//! Local single-run execution lifecycle and SQLite history.

mod catalog_store;
mod snapshot;
mod store;

use crate::{
    snapshot::{create_filtered_snapshot, validate_workspace},
    store::{RunFinish, Store},
};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use vocamesh_agent::execution::{
    classify_cli_stderr, ClaudeCli, ClaudeOutput, CliFailureKind, ExecutionEnvironment,
};
#[cfg(not(windows))]
use vocamesh_agent::AgentRuntime;
use vocamesh_core::{
    AddTeamMemberInput, AgentRole, AppearanceSettings, AuthenticationState, CatalogStatus,
    CreateAgentInput, CreateTeamInput, CreateWorkspaceInput, ExecutionReadiness, ExecutionRequest,
    ProcessGroupStatus, Provider, RecoveryInspection, RecoveryResolution, RecoveryStatus, RunEvent,
    RunRecord, RunState, ServiceError, SnapshotStatus, TaskRecord, TeamRecord, UnknownRunSummary,
    UpdateAgentInput, UpdateTeamInput, UpdateWorkspaceScopesInput, WorkspaceRecord,
};

const ERR_CLI: ServiceError = ServiceError::new(
    "execution.cli-not-ready",
    "Claude Code CLI 未通过安全版本检查",
);
const ERR_AUTH: ServiceError = ServiceError::new(
    "execution.auth-required",
    "Claude Code 尚未确认已登录，请先在 CLI 中完成登录",
);
const ERR_CONSENT: ServiceError = ServiceError::new(
    "execution.consent-required",
    "开始真实只读任务前必须确认目录与数据发送范围",
);
const ERR_RUNNING: ServiceError =
    ServiceError::new("execution.already-running", "当前已有一个本机 Run 正在执行");
const ERR_PROCESS_UNKNOWN: ServiceError = ServiceError::new(
    "execution.process-state-unknown",
    "先前 Claude Code 进程状态无法确认；为避免重复启动，真实执行已暂停",
);
const ERR_POLICY_UNVERIFIED: ServiceError = ServiceError::new(
    "execution.managed-policy-unverified",
    "无法确认 Claude Code 的组织托管 Hook 是否会产生额外副作用；真实执行已禁用",
);
const ERR_SETTINGS_INVALID: ServiceError = ServiceError::new("settings.invalid", "外观设置值无效");
const ERR_DUPLICATE: ServiceError =
    ServiceError::new("execution.request-duplicate", "该请求编号已用于其他任务");
const ERR_STOP: ServiceError = ServiceError::new("run.stop-failed", "无法向运行时发送停止请求");
const ERR_STOP_UNCONFIRMED: ServiceError = ServiceError::new(
    "run.stop-unconfirmed",
    "无法确认 Claude Code 进程树已经退出，Run 状态已设为未知",
);
const MAX_PROMPT_BYTES: usize = 64 * 1024;
const RUN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const STOP_GRACE: Duration = Duration::from_secs(2);
const STREAM_LINE_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub enum RuntimeNotification {
    State(RunRecord),
    Event(RunEvent),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPage {
    pub items: Vec<RunRecord>,
    pub has_more: bool,
    pub next_cursor: Option<u64>,
}

#[derive(Clone)]
pub struct ExecutionRuntime {
    store: Arc<Mutex<Store>>,
    config_dir: PathBuf,
    data_dir: PathBuf,
    active: Arc<Mutex<HashMap<String, mpsc::SyncSender<()>>>>,
    start_gate: Arc<Mutex<()>>,
    run_timeout: Duration,
    managed_policy: ManagedPolicyStatus,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ManagedPolicyStatus {
    Unverified,
    #[cfg(test)]
    TestVerifiedSafe,
}

impl ExecutionRuntime {
    pub fn open(
        config_dir: impl Into<PathBuf>,
        data_dir: impl Into<PathBuf>,
    ) -> Result<Self, ServiceError> {
        Self::open_with_timeout(config_dir, data_dir, RUN_TIMEOUT)
    }

    fn open_with_timeout(
        config_dir: impl Into<PathBuf>,
        data_dir: impl Into<PathBuf>,
        run_timeout: Duration,
    ) -> Result<Self, ServiceError> {
        Self::open_with_policy(
            config_dir,
            data_dir,
            run_timeout,
            ManagedPolicyStatus::Unverified,
        )
    }

    fn open_with_policy(
        config_dir: impl Into<PathBuf>,
        data_dir: impl Into<PathBuf>,
        run_timeout: Duration,
        managed_policy: ManagedPolicyStatus,
    ) -> Result<Self, ServiceError> {
        let config_dir = config_dir.into();
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)
            .map_err(|_| ServiceError::new("storage.write-failed", "本地任务历史无法保存"))?;
        let data_dir = std::fs::canonicalize(data_dir)
            .map_err(|_| ServiceError::new("storage.write-failed", "本地任务历史无法保存"))?;
        let store = Store::open(&data_dir.join("runs.sqlite3"))?;
        let runtime = Self {
            store: Arc::new(Mutex::new(store)),
            config_dir,
            data_dir,
            active: Arc::new(Mutex::new(HashMap::new())),
            start_gate: Arc::new(Mutex::new(())),
            run_timeout,
            managed_policy,
        };
        runtime
            .store
            .lock()
            .map_err(|_| storage_read())?
            .recover_active()?;
        Ok(runtime)
    }

    pub fn readiness(&self) -> ExecutionReadiness {
        let store = match self.store.lock() {
            Ok(store) => store,
            Err(_) => return not_ready("storage.read-failed", "本地任务历史无法读取"),
        };
        let uncertain = match store.has_uncertain_process() {
            Ok(value) => value,
            Err(error) => return not_ready(error.code, error.message),
        };
        let active = match store.has_active() {
            Ok(value) => value,
            Err(error) => return not_ready(error.code, error.message),
        };
        if uncertain {
            return not_ready(
                "execution.process-state-unknown",
                "先前进程状态无法确认；为避免重复启动，真实执行已暂停",
            );
        }
        if active {
            return not_ready(
                "execution.already-running",
                "本机 Run 尚未完成状态核对；真实执行已暂停",
            );
        }
        #[cfg(windows)]
        {
            return ExecutionReadiness {
                ready: false,
                version: None,
                auth_state: AuthenticationState::Unknown,
                message: "Windows 进程树停止策略尚未通过验证，真实执行暂不可用".into(),
                reason_code: Some("execution.platform-unsupported".into()),
            };
        }
        #[cfg(not(windows))]
        {
            if self.managed_policy == ManagedPolicyStatus::Unverified {
                return not_ready(ERR_POLICY_UNVERIFIED.code, ERR_POLICY_UNVERIFIED.message);
            }
            let tools = AgentRuntime::new(self.config_dir.clone());
            let Ok(path) = tools.executable_path(Provider::Claude) else {
                return not_ready("execution.cli-not-found", "未找到受支持的 Claude Code CLI");
            };
            let environment = ExecutionEnvironment::current();
            let Ok(cli) = ClaudeCli::verify_with_environment(&path, environment) else {
                return not_ready(
                    "execution.cli-incompatible",
                    "Claude Code CLI 版本或安全参数不兼容",
                );
            };
            let auth_state = cli.authentication_state();
            let ready = auth_state == AuthenticationState::Authenticated;
            let reason_code = match auth_state {
                AuthenticationState::Authenticated => None,
                AuthenticationState::Unauthenticated => Some("execution.auth-required".into()),
                AuthenticationState::Unknown | AuthenticationState::Unsupported => {
                    Some("execution.auth-unknown".into())
                }
            };
            let message = match auth_state {
                AuthenticationState::Authenticated => "Claude Code CLI 已登录，可开始单次只读任务",
                AuthenticationState::Unauthenticated => "Claude Code CLI 未登录，请先在 CLI 中登录",
                _ => "无法确认 Claude Code 登录状态，已阻止执行",
            };
            ExecutionReadiness {
                ready,
                version: Some(format!(
                    "{}.{}.{}",
                    cli.version().0,
                    cli.version().1,
                    cli.version().2
                )),
                auth_state,
                message: message.into(),
                reason_code,
            }
        }
    }

    /// Catalog operations share the execution history database and deliberately do not consult readiness.
    pub fn catalog_status(&self) -> Result<CatalogStatus, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_read())?
            .catalog_status()
    }

    pub fn list_agents(&self) -> Result<Vec<AgentRole>, ServiceError> {
        self.store.lock().map_err(|_| storage_read())?.list_agents()
    }

    pub fn get_agent(&self, id: &str) -> Result<AgentRole, ServiceError> {
        self.store.lock().map_err(|_| storage_read())?.get_agent(id)
    }

    pub fn create_agent(&self, input: CreateAgentInput) -> Result<AgentRole, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .create_agent(input)
    }

    pub fn update_agent(
        &self,
        id: &str,
        input: UpdateAgentInput,
    ) -> Result<AgentRole, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .update_agent(id, input)
    }

    pub fn archive_agent(&self, id: &str) -> Result<(), ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .archive_agent(id)
    }

    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceRecord>, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_read())?
            .list_workspaces()
    }

    pub fn get_workspace(&self, id: &str) -> Result<WorkspaceRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_read())?
            .get_workspace(id)
    }

    pub fn create_workspace(
        &self,
        input: CreateWorkspaceInput,
    ) -> Result<WorkspaceRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .create_workspace(input)
    }

    pub fn update_workspace_scopes(
        &self,
        id: &str,
        input: UpdateWorkspaceScopesInput,
    ) -> Result<WorkspaceRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .update_workspace_scopes(id, input)
    }

    pub fn rename_workspace(
        &self,
        id: &str,
        name: &str,
        expected_revision: Option<u64>,
    ) -> Result<WorkspaceRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .rename_workspace(id, name, expected_revision)
    }

    pub fn archive_workspace(&self, id: &str) -> Result<(), ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .archive_workspace(id)
    }

    pub fn list_teams(&self) -> Result<Vec<TeamRecord>, ServiceError> {
        self.store.lock().map_err(|_| storage_read())?.list_teams()
    }

    pub fn get_team(&self, id: &str) -> Result<TeamRecord, ServiceError> {
        self.store.lock().map_err(|_| storage_read())?.get_team(id)
    }

    pub fn create_team(&self, input: CreateTeamInput) -> Result<TeamRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .create_team(input)
    }

    pub fn update_team(
        &self,
        id: &str,
        input: UpdateTeamInput,
    ) -> Result<TeamRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .update_team(id, input)
    }

    pub fn add_team_member(
        &self,
        id: &str,
        input: AddTeamMemberInput,
    ) -> Result<TeamRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .add_team_member(id, input)
    }

    pub fn remove_team_member(&self, id: &str, agent_id: &str) -> Result<TeamRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .remove_team_member(id, agent_id)
    }

    pub fn set_team_reports_to(
        &self,
        id: &str,
        agent_id: &str,
        reports_to: &str,
    ) -> Result<TeamRecord, ServiceError> {
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .set_team_reports_to(id, agent_id, reports_to)
    }

    pub fn appearance_settings(&self) -> Result<AppearanceSettings, ServiceError> {
        let value = self
            .store
            .lock()
            .map_err(|_| storage_read())?
            .get_catalog_setting("appearance")?;
        value
            .map(|value| serde_json::from_str(&value).map_err(|_| storage_read()))
            .unwrap_or_else(|| Ok(AppearanceSettings::default()))
    }

    pub fn update_appearance_settings(
        &self,
        settings: AppearanceSettings,
    ) -> Result<AppearanceSettings, ServiceError> {
        if !matches!(settings.theme.as_str(), "system" | "light" | "dark")
            || !matches!(settings.density.as_str(), "comfortable" | "compact")
        {
            return Err(ERR_SETTINGS_INVALID);
        }
        let value = serde_json::to_string(&settings).map_err(|_| ERR_SETTINGS_INVALID)?;
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .set_catalog_setting("appearance", &value)?;
        Ok(settings)
    }

    #[cfg(windows)]
    pub fn start_read_only_task<F>(
        &self,
        _request: ExecutionRequest,
        _notify: F,
    ) -> Result<(TaskRecord, RunRecord), ServiceError>
    where
        F: Fn(RuntimeNotification) + Send + Sync + 'static,
    {
        Err(ERR_CLI)
    }

    #[cfg(not(windows))]
    pub fn start_read_only_task<F>(
        &self,
        request: ExecutionRequest,
        notify: F,
    ) -> Result<(TaskRecord, RunRecord), ServiceError>
    where
        F: Fn(RuntimeNotification) + Send + Sync + 'static,
    {
        let _gate = self.start_gate.lock().map_err(|_| ERR_RUNNING)?;
        validate_request(&request)?;
        {
            let store = self.store.lock().map_err(|_| storage_read())?;
            if let Some(existing) = store.has_request(&request.request_id)? {
                let (task, run) = existing;
                let requested_workspace = std::fs::canonicalize(&request.workspace_path)
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| request.workspace_path.clone());
                if task.title != request.title
                    || task.prompt != request.prompt
                    || task.workspace_path != requested_workspace
                {
                    return Err(ERR_DUPLICATE);
                }
                return Ok((task, run));
            }
            if store.has_active()? {
                return Err(ERR_RUNNING);
            }
            if store.has_uncertain_process()? {
                return Err(ERR_PROCESS_UNKNOWN);
            }
        }
        if self.managed_policy == ManagedPolicyStatus::Unverified {
            return Err(ERR_POLICY_UNVERIFIED);
        }
        let environment = ExecutionEnvironment::current();
        let executable = AgentRuntime::new(self.config_dir.clone())
            .executable_path(Provider::Claude)
            .map_err(|_| ERR_CLI)?;
        let cli = ClaudeCli::verify_with_environment(&executable, environment.clone())
            .map_err(|_| ERR_CLI)?;
        let auth = cli.authentication_state();
        if auth == AuthenticationState::Unauthenticated {
            return Err(ERR_AUTH);
        }
        if auth != AuthenticationState::Authenticated {
            return Err(ERR_CLI);
        }
        let workspace = validate_workspace(&request.workspace_path, &self.data_dir)?;
        let snapshot_path = self
            .data_dir
            .join("run-snapshots")
            .join(&request.request_id);
        create_filtered_snapshot(&workspace, &snapshot_path)?;
        let (task, run) = match self
            .store
            .lock()
            .map_err(|_| storage_write())?
            .create_pending(&request, &workspace.path.to_string_lossy())
        {
            Ok(records) => records,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&snapshot_path);
                return Err(error);
            }
        };
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        self.active
            .lock()
            .map_err(|_| ERR_RUNNING)?
            .insert(run.id.clone(), stop_tx);
        let cleanup_path = snapshot_path.clone();
        let worker = Worker {
            runtime: self.clone(),
            run_id: run.id.clone(),
            prompt: request.prompt,
            executable,
            environment,
            snapshot_path,
            stop_rx,
            notify: Arc::new(notify),
        };
        if thread::Builder::new()
            .name(format!("vocamesh-run-{}", &run.id[..run.id.len().min(12)]))
            .spawn(move || worker.execute())
            .is_err()
        {
            self.active.lock().map_err(|_| ERR_RUNNING)?.remove(&run.id);
            let mut store = self.store.lock().map_err(|_| storage_write())?;
            let finished = store.finish(
                &run.id,
                RunFinish {
                    run_state: RunState::Crashed,
                    outcome: "spawn-failed",
                    task_state: "failed",
                    exit_code: None,
                    summary: None,
                    error_code: Some("execution.spawn-failed"),
                    pid: None,
                },
            )?;
            if finished.state != RunState::Unknown {
                let _ = std::fs::remove_dir_all(cleanup_path);
            }
            return Ok((task, finished));
        }
        Ok((task, run))
    }

    pub fn stop_run<F>(&self, run_id: &str, notify: F) -> Result<RunRecord, ServiceError>
    where
        F: Fn(RuntimeNotification),
    {
        let current = self.get_run(run_id)?;
        if !current.state.is_active() {
            return Ok(current);
        }
        let state = self.store.lock().map_err(|_| storage_write())?.set_state(
            run_id,
            RunState::StopRequested,
            "stop-pending",
            None,
            None,
        )?;
        notify(RuntimeNotification::State(state.clone()));
        if !state.state.is_active() {
            return Ok(state);
        }
        let sender = self
            .active
            .lock()
            .map_err(|_| ERR_STOP)?
            .get(run_id)
            .cloned();
        if sender.as_ref().is_some_and(|sender| {
            matches!(
                sender.try_send(()),
                Ok(()) | Err(mpsc::TrySendError::Full(()))
            )
        }) {
            return Ok(state);
        }
        let unknown = self.store.lock().map_err(|_| storage_write())?.finish(
            run_id,
            RunFinish {
                run_state: RunState::Unknown,
                outcome: "unknown",
                task_state: "unknown",
                exit_code: None,
                summary: None,
                error_code: Some("run.stop-unconfirmed"),
                pid: current.process_id,
            },
        )?;
        notify(RuntimeNotification::State(unknown));
        Err(ERR_STOP_UNCONFIRMED)
    }

    pub fn shutdown(&self) {
        let run_ids = self
            .active
            .lock()
            .map(|active| active.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for run_id in &run_ids {
            let _ = self.stop_run(run_id, |_| {});
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self
                .active
                .lock()
                .map(|active| active.is_empty())
                .unwrap_or(false)
            {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        if let Ok(mut store) = self.store.lock() {
            for run_id in run_ids {
                let pid = store.get_run(&run_id).ok().and_then(|run| run.process_id);
                let _ = store.finish(
                    &run_id,
                    RunFinish {
                        run_state: RunState::Unknown,
                        outcome: "unknown",
                        task_state: "unknown",
                        exit_code: None,
                        summary: None,
                        error_code: Some("run.stop-unconfirmed"),
                        pid,
                    },
                );
            }
        }
    }

    pub fn get_run(&self, id: &str) -> Result<RunRecord, ServiceError> {
        self.store.lock().map_err(|_| storage_read())?.get_run(id)
    }

    pub fn list_unknown_runs(&self) -> Result<Vec<UnknownRunSummary>, ServiceError> {
        let _gate = self.start_gate.lock().map_err(|_| storage_write())?;
        let records = self
            .store
            .lock()
            .map_err(|_| storage_read())?
            .list_unknown_runs()?;
        Ok(records
            .into_iter()
            .map(|(run, task_title)| UnknownRunSummary {
                snapshot_status: self.snapshot_status(&run.request_id),
                run,
                task_title,
            })
            .collect())
    }

    pub fn inspect_unknown_run(&self, run_id: &str) -> Result<RecoveryInspection, ServiceError> {
        let _gate = self.start_gate.lock().map_err(|_| storage_write())?;
        validate_run_id(run_id)?;
        let process_group_id = {
            let store = self.store.lock().map_err(|_| storage_read())?;
            let run = store.get_run(run_id)?;
            if run.state != RunState::Unknown {
                return Err(ServiceError::new(
                    "run.recovery-not-required",
                    "该 Run 当前不需要恢复检查",
                ));
            }
            store.process_group_id(run_id)?
        };
        let process_group_status = inspect_process_group(process_group_id);
        let (run, task_title) = {
            let mut store = self.store.lock().map_err(|_| storage_write())?;
            store.inspect_unknown_run(run_id)?;
            let run = store.get_run(run_id)?;
            let task = store.get_task(&run.task_id)?;
            (run, task.title)
        };
        Ok(RecoveryInspection {
            snapshot_status: self.snapshot_status(&run.request_id),
            run,
            task_title,
            process_group_status,
        })
    }

    pub fn resolve_unknown_run(
        &self,
        run_id: &str,
        user_confirmed: bool,
    ) -> Result<RecoveryResolution, ServiceError> {
        let _gate = self.start_gate.lock().map_err(|_| storage_write())?;
        validate_run_id(run_id)?;
        if !user_confirmed {
            return Err(ServiceError::new(
                "run.recovery-confirmation-required",
                "请先在系统进程管理工具中核验并确认相关进程已退出",
            ));
        }
        let (run, process_group_id) = {
            let store = self.store.lock().map_err(|_| storage_read())?;
            let run = store.get_run(run_id)?;
            (run, store.process_group_id(run_id)?)
        };
        if run.state != RunState::Unknown || run.recovery_status != RecoveryStatus::Required {
            return Err(ServiceError::new(
                "run.recovery-not-required",
                "该 Run 当前不满足归档条件",
            ));
        }
        if inspect_process_group(process_group_id) != ProcessGroupStatus::ExitedCandidate {
            return Err(ServiceError::new(
                "run.recovery-process-present",
                "后端未能确认原进程组已退出；Run 和快照仍保留",
            ));
        }
        let _snapshot = self.snapshot_path(&run.request_id)?;
        self.store
            .lock()
            .map_err(|_| storage_write())?
            .resolve_unknown_run(run_id)?;
        let cleanup_failed =
            snapshot::remove_snapshot(&self.data_dir.join("run-snapshots"), &run.request_id)
                .is_err();
        if cleanup_failed {
            self.store
                .lock()
                .map_err(|_| storage_write())?
                .record_cleanup_failure(run_id)?;
        }
        Ok(RecoveryResolution {
            run: self.get_run(run_id)?,
            snapshot_status: self.snapshot_status(&run.request_id),
        })
    }

    fn snapshot_status(&self, request_id: &str) -> SnapshotStatus {
        match self.snapshot_path(request_id) {
            Ok(path) => match std::fs::symlink_metadata(path) {
                Ok(metadata)
                    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() =>
                {
                    SnapshotStatus::Exists
                }
                Ok(_) => SnapshotStatus::CannotVerify,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    SnapshotStatus::Missing
                }
                Err(_) => SnapshotStatus::CannotVerify,
            },
            Err(_) => SnapshotStatus::CannotVerify,
        }
    }

    fn snapshot_path(&self, request_id: &str) -> Result<PathBuf, ServiceError> {
        if !valid_request_id(request_id) {
            return Err(ServiceError::new(
                "run.recovery-path-invalid",
                "该 Run 的快照标识无法安全核验",
            ));
        }
        let root = self.data_dir.join("run-snapshots");
        match std::fs::symlink_metadata(&root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(root.join(request_id))
            }
            Err(_) => {
                return Err(ServiceError::new(
                    "run.recovery-path-invalid",
                    "快照目录无法安全核验",
                ))
            }
            Ok(metadata) if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() => {
                return Err(ServiceError::new(
                    "run.recovery-path-invalid",
                    "快照目录无法安全核验",
                ));
            }
            Ok(_) => {}
        }
        let canonical_root = std::fs::canonicalize(&root)
            .map_err(|_| ServiceError::new("run.recovery-path-invalid", "快照目录无法安全核验"))?;
        if canonical_root != root {
            return Err(ServiceError::new(
                "run.recovery-path-invalid",
                "快照目录无法安全核验",
            ));
        }
        let path = canonical_root.join(request_id);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err(ServiceError::new(
                "run.recovery-path-invalid",
                "快照对象无法安全核验",
            )),
            Ok(_) => {
                let canonical = std::fs::canonicalize(&path).map_err(|_| {
                    ServiceError::new("run.recovery-path-invalid", "快照对象无法安全核验")
                })?;
                if canonical.parent() != Some(canonical_root.as_path()) {
                    return Err(ServiceError::new(
                        "run.recovery-path-invalid",
                        "快照对象超出受信任目录",
                    ));
                }
                Ok(canonical)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
            Err(_) => Err(ServiceError::new(
                "run.recovery-path-invalid",
                "快照对象无法安全核验",
            )),
        }
    }
    pub fn get_task(&self, id: &str) -> Result<TaskRecord, ServiceError> {
        self.store.lock().map_err(|_| storage_read())?.get_task(id)
    }
    pub fn list_tasks(&self) -> Result<Vec<TaskRecord>, ServiceError> {
        self.store.lock().map_err(|_| storage_read())?.list_tasks()
    }
    pub fn list_runs(&self, cursor: Option<u64>, limit: usize) -> Result<RunPage, ServiceError> {
        let (items, has_more, next_cursor) = self
            .store
            .lock()
            .map_err(|_| storage_read())?
            .list_runs(cursor, limit)?;
        Ok(RunPage {
            items,
            has_more,
            next_cursor,
        })
    }
    pub fn list_run_events(
        &self,
        run_id: &str,
        after: u64,
        limit: usize,
    ) -> Result<Vec<RunEvent>, ServiceError> {
        self.get_run(run_id)?;
        self.store
            .lock()
            .map_err(|_| storage_read())?
            .list_events(run_id, after, limit)
    }
}

struct Worker {
    runtime: ExecutionRuntime,
    run_id: String,
    prompt: String,
    executable: PathBuf,
    environment: ExecutionEnvironment,
    snapshot_path: PathBuf,
    stop_rx: mpsc::Receiver<()>,
    notify: Arc<dyn Fn(RuntimeNotification) + Send + Sync>,
}

enum ProcessCompletion {
    ExitedSuccessfully {
        exit_code: Option<i32>,
        summary: String,
    },
    ExitedWithFailure {
        outcome: &'static str,
        exit_code: Option<i32>,
        summary: Option<String>,
        error_code: &'static str,
    },
    StoppedAndExited {
        exit_code: Option<i32>,
    },
    TimedOutAndExited {
        exit_code: Option<i32>,
    },
    ExitUnconfirmed {
        error_code: &'static str,
        pid: Option<u32>,
    },
}

impl Worker {
    fn execute(self) {
        let completion = self.run_process();
        self.finalize(completion);
    }

    fn finalize(&self, completion: ProcessCompletion) {
        let completion_pid = match &completion {
            ProcessCompletion::ExitUnconfirmed { pid, .. } => *pid,
            _ => None,
        };
        let (run_state, outcome, task_state, exit_code, summary, error_code, uncertain) =
            match &completion {
                ProcessCompletion::ExitedSuccessfully { exit_code, summary } => (
                    RunState::Finished,
                    "success",
                    "done",
                    *exit_code,
                    Some(summary.as_str()),
                    None,
                    false,
                ),
                ProcessCompletion::ExitedWithFailure {
                    outcome,
                    exit_code,
                    summary,
                    error_code,
                } => (
                    RunState::Crashed,
                    *outcome,
                    "failed",
                    *exit_code,
                    summary.as_deref(),
                    Some(*error_code),
                    false,
                ),
                ProcessCompletion::StoppedAndExited { exit_code } => (
                    RunState::Finished,
                    "stopped",
                    "stopped",
                    *exit_code,
                    None,
                    None,
                    false,
                ),
                ProcessCompletion::TimedOutAndExited { exit_code } => (
                    RunState::Finished,
                    "timeout",
                    "failed",
                    *exit_code,
                    None,
                    Some("run.timeout"),
                    false,
                ),
                ProcessCompletion::ExitUnconfirmed { error_code, .. } => (
                    RunState::Unknown,
                    "unknown",
                    "unknown",
                    None,
                    None,
                    Some(*error_code),
                    true,
                ),
            };

        let persisted = self.runtime.store.lock().ok().and_then(|mut store| {
            match store.finish(
                &self.run_id,
                RunFinish {
                    run_state,
                    outcome,
                    task_state,
                    exit_code,
                    summary,
                    error_code,
                    pid: completion_pid,
                },
            ) {
                Ok(run) => Some(run),
                Err(_) => store
                    .finish(
                        &self.run_id,
                        RunFinish {
                            run_state: RunState::Unknown,
                            outcome: "unknown",
                            task_state: "unknown",
                            exit_code: None,
                            summary: None,
                            error_code: Some("storage.write-failed"),
                            pid: completion_pid,
                        },
                    )
                    .ok(),
            }
        });

        let can_cleanup = !uncertain
            && persisted
                .as_ref()
                .is_some_and(|run| run.state != RunState::Unknown);
        if let Some(run) = persisted {
            (self.notify)(RuntimeNotification::State(run));
        }
        if can_cleanup {
            let _ = std::fs::remove_dir_all(&self.snapshot_path);
        }
        if let Ok(mut active) = self.runtime.active.lock() {
            active.remove(&self.run_id);
        }
    }

    fn run_process(&self) -> ProcessCompletion {
        if self.stop_rx.try_recv().is_ok() {
            return ProcessCompletion::StoppedAndExited { exit_code: None };
        }
        let cli =
            match ClaudeCli::verify_with_environment(&self.executable, self.environment.clone()) {
                Ok(cli) => cli,
                Err(_) => return failure("execution.cli-not-ready"),
            };
        if cli.authentication_state() != AuthenticationState::Authenticated {
            return failure("execution.auth-required");
        }
        let starting = match self.runtime.store.lock() {
            Ok(mut store) => {
                match store.set_state(&self.run_id, RunState::Starting, "starting", None, None) {
                    Ok(run) => run,
                    Err(_) => return failure("storage.write-failed"),
                }
            }
            Err(_) => return failure("storage.write-failed"),
        };
        (self.notify)(RuntimeNotification::State(starting));
        if self.stop_rx.try_recv().is_ok() {
            return ProcessCompletion::StoppedAndExited { exit_code: None };
        }
        match self.runtime.get_run(&self.run_id) {
            Ok(run) if run.state == RunState::StopRequested => {
                return ProcessCompletion::StoppedAndExited { exit_code: None };
            }
            Err(_) => return failure("storage.read-failed"),
            _ => {}
        }
        let mut command = Command::new(&self.executable);
        command
            .args(ClaudeCli::read_only_args(&self.prompt))
            .current_dir(&self.snapshot_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cli.configure_command(&mut command);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => return failure("execution.spawn-failed"),
        };
        let pid = child.id();
        self.monitor_process(&mut child, pid)
    }

    fn monitor_process(&self, child: &mut Child, pid: u32) -> ProcessCompletion {
        let running = match self.runtime.store.lock() {
            Ok(mut store) => {
                match store.set_state(
                    &self.run_id,
                    RunState::Running,
                    "running",
                    Some(pid),
                    process_group_id_for(pid),
                ) {
                    Ok(run) => run,
                    Err(_) => {
                        return if terminate_process_tree(child, pid) {
                            failure("storage.write-failed")
                        } else {
                            ProcessCompletion::ExitUnconfirmed {
                                error_code: "run.stop-unconfirmed",
                                pid: Some(pid),
                            }
                        };
                    }
                }
            }
            Err(_) => {
                return if terminate_process_tree(child, pid) {
                    failure("storage.write-failed")
                } else {
                    ProcessCompletion::ExitUnconfirmed {
                        error_code: "run.stop-unconfirmed",
                        pid: Some(pid),
                    }
                };
            }
        };
        (self.notify)(RuntimeNotification::State(running));
        let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
            return if terminate_process_tree(child, pid) {
                failure("execution.output-protocol-error")
            } else {
                ProcessCompletion::ExitUnconfirmed {
                    error_code: "run.stop-unconfirmed",
                    pid: Some(pid),
                }
            };
        };
        let (output_tx, output_rx) = mpsc::sync_channel::<ClaudeOutput>(128);
        let stdout_tx = output_tx.clone();
        thread::spawn(move || read_stream(stdout, stdout_tx));
        let (stderr_tx, stderr_rx) = mpsc::channel();
        thread::spawn(move || {
            let bounded = vocamesh_agent::execution::read_bounded_stderr(stderr);
            let _ = stderr_tx.send(classify_cli_stderr(&bounded));
        });
        drop(output_tx);

        let start = Instant::now();
        let mut stop_requested = false;
        let mut timed_out = false;
        let mut protocol_error = false;
        let mut final_summary: Option<String> = None;
        let exit_status = loop {
            if !stop_requested && self.stop_rx.try_recv().is_ok() {
                stop_requested = true;
                if !terminate_process_tree(child, pid) {
                    return ProcessCompletion::ExitUnconfirmed {
                        error_code: "run.stop-unconfirmed",
                        pid: Some(pid),
                    };
                }
            }
            if !timed_out && start.elapsed() >= self.runtime.run_timeout {
                timed_out = true;
                if !terminate_process_tree(child, pid) {
                    return ProcessCompletion::ExitUnconfirmed {
                        error_code: "run.stop-unconfirmed",
                        pid: Some(pid),
                    };
                }
            }
            if drain_outputs(
                &self.runtime,
                &self.run_id,
                &output_rx,
                &self.notify,
                &mut final_summary,
                &mut protocol_error,
            )
            .is_err()
            {
                return if terminate_process_tree(child, pid) {
                    failure("storage.write-failed")
                } else {
                    ProcessCompletion::ExitUnconfirmed {
                        error_code: "run.stop-unconfirmed",
                        pid: Some(pid),
                    }
                };
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    if process_group_exists(-(pid as i32)) {
                        if !terminate_process_tree(child, pid) {
                            return ProcessCompletion::ExitUnconfirmed {
                                error_code: "run.stop-unconfirmed",
                                pid: Some(pid),
                            };
                        }
                        return ProcessCompletion::ExitedWithFailure {
                            outcome: "child-process-remained",
                            exit_code: status.code(),
                            summary: None,
                            error_code: "execution.child-process-remained",
                        };
                    }
                    break status;
                }
                Ok(None) => thread::sleep(Duration::from_millis(25)),
                Err(_) => {
                    let _ = terminate_process_tree(child, pid);
                    return ProcessCompletion::ExitUnconfirmed {
                        error_code: "execution.process-status-unknown",
                        pid: Some(pid),
                    };
                }
            }
        };
        let drain_deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < drain_deadline {
            match output_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(output) => {
                    if self
                        .store_output(output, &mut final_summary, &mut protocol_error)
                        .is_err()
                    {
                        return failure("storage.write-failed");
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
        let code = exit_status.code();
        let stderr_kind = stderr_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or(CliFailureKind::Other);
        if stop_requested {
            ProcessCompletion::StoppedAndExited { exit_code: code }
        } else if timed_out {
            ProcessCompletion::TimedOutAndExited { exit_code: code }
        } else if protocol_error {
            ProcessCompletion::ExitedWithFailure {
                outcome: "output-protocol-error",
                exit_code: code,
                summary: final_summary,
                error_code: "run.output-protocol-error",
            }
        } else if !exit_status.success() {
            ProcessCompletion::ExitedWithFailure {
                outcome: "failed",
                exit_code: code,
                summary: final_summary,
                error_code: stderr_kind.error_code(),
            }
        } else if final_summary.is_none() {
            ProcessCompletion::ExitedWithFailure {
                outcome: "missing-final-result",
                exit_code: code,
                summary: None,
                error_code: "run.output-protocol-error",
            }
        } else {
            ProcessCompletion::ExitedSuccessfully {
                exit_code: code,
                summary: final_summary.unwrap_or_default(),
            }
        }
    }

    fn store_output(
        &self,
        output: ClaudeOutput,
        summary: &mut Option<String>,
        protocol_error: &mut bool,
    ) -> Result<(), &'static str> {
        let (kind, text) = match output {
            ClaudeOutput::TextDelta(text) => ("text", text),
            ClaudeOutput::Final(text) => {
                *summary = Some(text.clone());
                ("final", text)
            }
            ClaudeOutput::ToolProgress(text) => ("progress", text.to_owned()),
            ClaudeOutput::ProtocolError => {
                *protocol_error = true;
                (
                    "protocol-error",
                    "无法解析 Claude Code 输出事件，原始内容已丢弃".into(),
                )
            }
            ClaudeOutput::Ignore => return Ok(()),
        };
        let event = self
            .runtime
            .store
            .lock()
            .map_err(|_| "storage.write-failed")?
            .append_event(&self.run_id, kind, &text)
            .map_err(|_| "storage.write-failed")?;
        if let Some(event) = event {
            (self.notify)(RuntimeNotification::Event(event));
        }
        Ok(())
    }
}

fn failure(error_code: &'static str) -> ProcessCompletion {
    ProcessCompletion::ExitedWithFailure {
        outcome: "failed",
        exit_code: None,
        summary: None,
        error_code,
    }
}

fn drain_outputs(
    runtime: &ExecutionRuntime,
    run_id: &str,
    rx: &mpsc::Receiver<ClaudeOutput>,
    notify: &Arc<dyn Fn(RuntimeNotification) + Send + Sync>,
    summary: &mut Option<String>,
    protocol_error: &mut bool,
) -> Result<(), &'static str> {
    while let Ok(output) = rx.try_recv() {
        let (kind, text) = match output {
            ClaudeOutput::TextDelta(text) => ("text", text),
            ClaudeOutput::Final(text) => {
                *summary = Some(text.clone());
                ("final", text)
            }
            ClaudeOutput::ToolProgress(text) => ("progress", text.to_owned()),
            ClaudeOutput::ProtocolError => {
                *protocol_error = true;
                (
                    "protocol-error",
                    "无法解析 Claude Code 输出事件，原始内容已丢弃".into(),
                )
            }
            ClaudeOutput::Ignore => continue,
        };
        let event = runtime
            .store
            .lock()
            .map_err(|_| "storage.write-failed")?
            .append_event(run_id, kind, &text)
            .map_err(|_| "storage.write-failed")?;
        if let Some(event) = event {
            notify(RuntimeNotification::Event(event));
        }
    }
    Ok(())
}

fn read_stream<R: std::io::Read>(mut reader: R, tx: mpsc::SyncSender<ClaudeOutput>) {
    let mut buffer = [0u8; 8192];
    let mut line = Vec::with_capacity(1024);
    let mut too_long = false;
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => break,
            Err(_) => {
                let _ = tx.send(ClaudeOutput::ProtocolError);
                return;
            }
            Ok(count) => count,
        };
        for byte in &buffer[..count] {
            if *byte == b'\n' {
                let event = if too_long {
                    ClaudeOutput::ProtocolError
                } else {
                    vocamesh_agent::execution::parse_stream_json(&line)
                };
                if tx.send(event).is_err() {
                    return;
                }
                line.clear();
                too_long = false;
            } else if line.len() < STREAM_LINE_LIMIT {
                line.push(*byte);
            } else {
                too_long = true;
            }
        }
    }
    if !line.is_empty() {
        let output = if too_long {
            ClaudeOutput::ProtocolError
        } else {
            vocamesh_agent::execution::parse_stream_json(&line)
        };
        let _ = tx.send(output);
    }
}

#[cfg(unix)]
fn terminate_process_tree(child: &mut Child, pid: u32) -> bool {
    let group = -(pid as i32);
    unsafe {
        libc::kill(group, libc::SIGTERM);
    }
    let deadline = Instant::now() + STOP_GRACE;
    while Instant::now() < deadline {
        let _ = child.try_wait();
        if !process_group_exists(group) {
            return true;
        }
        thread::sleep(Duration::from_millis(25));
    }
    unsafe {
        libc::kill(group, libc::SIGKILL);
    }
    let deadline = Instant::now() + STOP_GRACE;
    while Instant::now() < deadline {
        let _ = child.try_wait();
        if !process_group_exists(group) {
            let _ = child.wait();
            return true;
        }
        thread::sleep(Duration::from_millis(25));
    }
    false
}

#[cfg(unix)]
fn process_group_exists(group: i32) -> bool {
    let result = unsafe { libc::kill(group, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
fn process_group_exists(_group: i32) -> bool {
    false
}

#[cfg(windows)]
fn terminate_process_tree(child: &mut Child, _pid: u32) -> bool {
    child.kill().is_ok() && child.wait().is_ok()
}

fn validate_request(request: &ExecutionRequest) -> Result<(), ServiceError> {
    if request.provider_id != Provider::Claude.id() {
        return Err(ServiceError::new(
            "execution.provider-not-supported",
            "本阶段只支持 Claude Code 只读执行",
        ));
    }
    if !request.consent {
        return Err(ERR_CONSENT);
    }
    if request.title.trim().is_empty()
        || request.title.len() > 256
        || request.prompt.trim().is_empty()
        || request.prompt.len() > MAX_PROMPT_BYTES
        || !valid_request_id(&request.request_id)
    {
        return Err(ServiceError::new(
            "execution.invalid-request",
            "任务标题、Prompt 或请求编号无效",
        ));
    }
    Ok(())
}

fn valid_request_id(value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    chars.len() == 36
        && [8, 13, 18, 23].iter().all(|&i| chars[i] == '-')
        && chars
            .iter()
            .enumerate()
            .all(|(i, c)| [8, 13, 18, 23].contains(&i) || c.is_ascii_hexdigit())
}

fn not_ready(reason_code: &str, message: &str) -> ExecutionReadiness {
    ExecutionReadiness {
        ready: false,
        version: None,
        auth_state: AuthenticationState::Unknown,
        message: message.into(),
        reason_code: Some(reason_code.into()),
    }
}

fn validate_run_id(value: &str) -> Result<(), ServiceError> {
    let valid = value.strip_prefix("run_").is_some_and(|suffix| {
        suffix.len() == 32 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
    });
    if valid {
        Ok(())
    } else {
        Err(ServiceError::new("run.invalid-id", "Run 编号无效"))
    }
}

#[cfg(unix)]
fn process_group_id_for(pid: u32) -> Option<u32> {
    Some(pid)
}

#[cfg(not(unix))]
fn process_group_id_for(_pid: u32) -> Option<u32> {
    None
}

fn inspect_process_group(process_group_id: Option<i64>) -> ProcessGroupStatus {
    let Some(group_id) = process_group_id.and_then(|value| i32::try_from(value).ok()) else {
        return ProcessGroupStatus::ManualRequired;
    };
    if group_id <= 0 {
        return ProcessGroupStatus::ManualRequired;
    }
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(-group_id, 0) };
        let errno = (result == -1)
            .then(|| std::io::Error::last_os_error().raw_os_error())
            .flatten();
        classify_process_group_probe(result, errno)
    }
    #[cfg(not(unix))]
    {
        let _ = group_id;
        ProcessGroupStatus::CannotVerify
    }
}

#[cfg(unix)]
fn classify_process_group_probe(result: i32, errno: Option<i32>) -> ProcessGroupStatus {
    if result == 0 {
        ProcessGroupStatus::StillRunning
    } else {
        match errno {
            Some(libc::ESRCH) => ProcessGroupStatus::ExitedCandidate,
            Some(libc::EPERM) | None => ProcessGroupStatus::CannotVerify,
            Some(_) => ProcessGroupStatus::CannotVerify,
        }
    }
}
fn storage_read() -> ServiceError {
    ServiceError::new("storage.read-failed", "本地任务历史无法读取")
}
fn storage_write() -> ServiceError {
    ServiceError::new("storage.write-failed", "本地任务历史无法保存")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn request(id: &str) -> ExecutionRequest {
        ExecutionRequest {
            request_id: id.into(),
            title: "Read only review".into(),
            prompt: "Summarize the source tree".into(),
            provider_id: "claude".into(),
            workspace_path: "/tmp/workspace".into(),
            consent: true,
        }
    }

    #[cfg(unix)]
    fn fake_runtime(root: &std::path::Path) -> ExecutionRuntime {
        fake_runtime_with_timeout(root, RUN_TIMEOUT)
    }

    #[cfg(unix)]
    fn fake_runtime_with_timeout(
        root: &std::path::Path,
        run_timeout: Duration,
    ) -> ExecutionRuntime {
        use std::os::unix::fs::PermissionsExt;

        let config = root.join("config");
        let data = root.join("data");
        std::fs::create_dir_all(&config).unwrap();
        let executable = root.join("fake-claude");
        std::fs::write(
            &executable,
            r##"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf '2.1.296 (Claude Code)\n'
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"authMethod":"oauth_token"}\n'
  exit 0
fi
case "$*" in
  *slow*)
    printf '{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"started"}}}\n'
    sleep 30
    ;;
  *fail*)
    printf '{"type":"result","result":"failed"}\n'
    exit 7
    ;;
  *diag*)
    printf 'unknown option token=sk-ant-test-secret /Users/private/person\n' >&2
    i=0
    while [ "$i" -lt 8000 ]; do printf x >&2; i=$((i + 1)); done
    exit 23
    ;;
  *orphan*)
    sleep 30 &
    printf '{"type":"result","result":"orphan started"}\n'
    exit 0
    ;;
  *protocol*)
    printf 'not-json\n'
    ;;
  *)
    printf '{"type":"result","result":"fake result"}\n'
    ;;
esac
"##,
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions).unwrap();
        std::fs::write(
            config.join("runtime-settings.json"),
            serde_json::json!({
                "schemaVersion": 1,
                "manualPaths": { "claude": executable },
                "defaultToolId": null,
            })
            .to_string(),
        )
        .unwrap();
        ExecutionRuntime::open_with_policy(
            config,
            data,
            run_timeout,
            ManagedPolicyStatus::TestVerifiedSafe,
        )
        .unwrap()
    }

    #[cfg(unix)]
    fn runtime_request(id: &str, workspace: &std::path::Path, prompt: &str) -> ExecutionRequest {
        ExecutionRequest {
            request_id: id.into(),
            title: "Fake Claude lifecycle test".into(),
            prompt: prompt.into(),
            provider_id: "claude".into(),
            workspace_path: workspace.to_string_lossy().into_owned(),
            consent: true,
        }
    }

    #[cfg(unix)]
    fn wait_for_terminal(runtime: &ExecutionRuntime, run_id: &str) -> RunRecord {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let run = runtime.get_run(run_id).unwrap();
            if !run.state.is_active() {
                return run;
            }
            assert!(Instant::now() < deadline, "fake Claude run did not finish");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(unix)]
    fn pending_running_run(runtime: &ExecutionRuntime, request_id: &str, pid: u32) -> RunRecord {
        pending_running_run_with_group(runtime, request_id, pid, Some(pid))
    }

    #[cfg(unix)]
    fn pending_running_run_with_group(
        runtime: &ExecutionRuntime,
        request_id: &str,
        pid: u32,
        process_group_id: Option<u32>,
    ) -> RunRecord {
        let mut store = runtime.store.lock().unwrap();
        let (_, run) = store
            .create_pending(&request(request_id), "/tmp/synthetic-workspace")
            .unwrap();
        store
            .set_state(&run.id, RunState::Starting, "starting", None, None)
            .unwrap();
        store
            .set_state(
                &run.id,
                RunState::Running,
                "running",
                Some(pid),
                process_group_id,
            )
            .unwrap()
    }

    #[cfg(unix)]
    fn unknown_run_with_group(
        runtime: &ExecutionRuntime,
        request_id: &str,
        pid: u32,
        process_group_id: Option<u32>,
    ) -> (RunRecord, PathBuf) {
        let run = pending_running_run_with_group(runtime, request_id, pid, process_group_id);
        let snapshot = runtime.data_dir.join("run-snapshots").join(&run.request_id);
        std::fs::create_dir_all(&snapshot).unwrap();
        std::fs::write(snapshot.join("README.md"), "synthetic").unwrap();
        worker_for(runtime, &run.id, snapshot.clone()).finalize(
            ProcessCompletion::ExitUnconfirmed {
                error_code: "run.stop-unconfirmed",
                pid: Some(pid),
            },
        );
        (runtime.get_run(&run.id).unwrap(), snapshot)
    }

    #[cfg(unix)]
    fn exited_test_process_group() -> u32 {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg("exit 0").process_group(0);
        let mut child = command.spawn().unwrap();
        let group_id = child.id();
        assert!(child.wait().unwrap().success());
        group_id
    }

    #[cfg(unix)]
    fn worker_for(runtime: &ExecutionRuntime, run_id: &str, snapshot_path: PathBuf) -> Worker {
        let (_stop_tx, stop_rx) = mpsc::sync_channel(1);
        Worker {
            runtime: runtime.clone(),
            run_id: run_id.to_owned(),
            prompt: "synthetic test".into(),
            executable: PathBuf::new(),
            environment: ExecutionEnvironment::current(),
            snapshot_path,
            stop_rx,
            notify: Arc::new(|_| {}),
        }
    }

    #[test]
    fn managed_policy_uncertainty_blocks_readiness_and_task_start() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let config = root.path().join("config");
        let data = root.path().join("data");
        std::fs::create_dir(&config).unwrap();
        let runtime = ExecutionRuntime::open_with_timeout(config, data, RUN_TIMEOUT).unwrap();

        let readiness = runtime.readiness();
        assert!(!readiness.ready);
        assert_eq!(
            readiness.reason_code.as_deref(),
            Some("execution.managed-policy-unverified")
        );
        assert_eq!(readiness.auth_state, AuthenticationState::Unknown);
        assert!(readiness.message.contains("托管 Hook"));
        let error = runtime
            .start_read_only_task(
                runtime_request(
                    "00000000-0000-4000-8000-000000000190",
                    workspace.path(),
                    "synthetic prompt",
                ),
                |_| {},
            )
            .unwrap_err();
        assert_eq!(error.code, "execution.managed-policy-unverified");
        assert!(runtime.list_tasks().unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn process_group_probe_classifies_only_esrch_as_exit_candidate() {
        assert_eq!(
            classify_process_group_probe(0, None),
            ProcessGroupStatus::StillRunning
        );
        assert_eq!(
            classify_process_group_probe(-1, Some(libc::ESRCH)),
            ProcessGroupStatus::ExitedCandidate
        );
        assert_eq!(
            classify_process_group_probe(-1, Some(libc::EPERM)),
            ProcessGroupStatus::CannotVerify
        );
        assert_eq!(
            classify_process_group_probe(-1, Some(libc::EIO)),
            ProcessGroupStatus::CannotVerify
        );
        assert_eq!(
            inspect_process_group(None),
            ProcessGroupStatus::ManualRequired
        );
    }

    #[cfg(unix)]
    #[test]
    fn unknown_run_with_live_process_group_keeps_snapshot_and_blocks_archive() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let group_id = unsafe { libc::getpgrp() as u32 };
        let (run, snapshot) = unknown_run_with_group(
            &runtime,
            "00000000-0000-4000-8000-000000000191",
            42_491,
            Some(group_id),
        );

        let inspected = runtime.inspect_unknown_run(&run.id).unwrap();
        assert_eq!(
            inspected.process_group_status,
            ProcessGroupStatus::StillRunning
        );
        assert_eq!(
            runtime.resolve_unknown_run(&run.id, true).unwrap_err().code,
            "run.recovery-process-present"
        );
        assert_eq!(runtime.get_run(&run.id).unwrap().state, RunState::Unknown);
        assert!(snapshot.join("README.md").exists());
        assert_eq!(
            runtime.readiness().reason_code.as_deref(),
            Some("execution.process-state-unknown")
        );
    }

    #[cfg(unix)]
    #[test]
    fn exited_candidate_requires_user_confirmation_and_archives_as_failure() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let (run, snapshot) = unknown_run_with_group(
            &runtime,
            "00000000-0000-4000-8000-000000000192",
            42_492,
            Some(exited_test_process_group()),
        );

        let inspected = runtime.inspect_unknown_run(&run.id).unwrap();
        assert_eq!(
            inspected.process_group_status,
            ProcessGroupStatus::ExitedCandidate
        );
        assert!(inspected.run.recovery_checked_at.is_some());
        assert_eq!(
            runtime
                .resolve_unknown_run(&run.id, false)
                .unwrap_err()
                .code,
            "run.recovery-confirmation-required"
        );
        assert_eq!(runtime.get_run(&run.id).unwrap().state, RunState::Unknown);
        assert!(snapshot.join("README.md").exists());

        let archived = runtime.resolve_unknown_run(&run.id, true).unwrap();
        assert_eq!(archived.run.state, RunState::Crashed);
        assert_eq!(
            archived.run.outcome.as_deref(),
            Some("recovered-after-exit")
        );
        assert_eq!(
            archived.run.error_code.as_deref(),
            Some("run.stop-unconfirmed")
        );
        assert_eq!(archived.run.recovery_status, RecoveryStatus::Resolved);
        assert_eq!(archived.snapshot_status, SnapshotStatus::Missing);
        assert_eq!(
            runtime.get_task(&archived.run.task_id).unwrap().state,
            "failed"
        );
        assert!(!snapshot.exists());
        let production_runtime =
            ExecutionRuntime::open(runtime.config_dir.clone(), runtime.data_dir.clone()).unwrap();
        assert_eq!(
            production_runtime.readiness().reason_code.as_deref(),
            Some("execution.managed-policy-unverified")
        );
        let events = runtime.list_run_events(&run.id, 0, 20).unwrap();
        assert!(events
            .iter()
            .any(|event| event.kind == "recovery-confirmed-exited"));
    }

    #[cfg(unix)]
    #[test]
    fn missing_process_group_requires_manual_handling() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let (run, snapshot) = unknown_run_with_group(
            &runtime,
            "00000000-0000-4000-8000-000000000193",
            42_493,
            None,
        );

        let inspected = runtime.inspect_unknown_run(&run.id).unwrap();
        assert_eq!(
            inspected.process_group_status,
            ProcessGroupStatus::ManualRequired
        );
        assert_eq!(
            inspected.run.recovery_status,
            RecoveryStatus::ManualRequired
        );
        assert_eq!(
            runtime.resolve_unknown_run(&run.id, true).unwrap_err().code,
            "run.recovery-not-required"
        );
        assert_eq!(runtime.get_run(&run.id).unwrap().state, RunState::Unknown);
        assert!(snapshot.join("README.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_failure_keeps_resolved_failure_and_records_fixed_event() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let (run, snapshot) = unknown_run_with_group(
            &runtime,
            "00000000-0000-4000-8000-000000000194",
            42_494,
            Some(exited_test_process_group()),
        );
        std::fs::remove_dir_all(&snapshot).unwrap();
        std::fs::write(&snapshot, "non-directory cleanup obstacle").unwrap();

        let archived = runtime.resolve_unknown_run(&run.id, true).unwrap();
        assert_eq!(archived.run.state, RunState::Crashed);
        assert_eq!(
            archived.run.outcome.as_deref(),
            Some("recovered-after-exit")
        );
        assert_eq!(archived.snapshot_status, SnapshotStatus::CannotVerify);
        let events = runtime.list_run_events(&run.id, 0, 20).unwrap();
        assert!(events
            .iter()
            .any(|event| event.kind == "recovery-snapshot-cleanup-failed"));
        assert_eq!(
            std::fs::read_to_string(snapshot).unwrap(),
            "non-directory cleanup obstacle"
        );
    }

    #[cfg(unix)]
    #[test]
    fn recovery_audit_write_failure_rolls_back_archive_and_keeps_snapshot() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let (run, snapshot) = unknown_run_with_group(
            &runtime,
            "00000000-0000-4000-8000-000000000195",
            42_495,
            Some(exited_test_process_group()),
        );
        rusqlite::Connection::open(runtime.data_dir.join("runs.sqlite3"))
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_recovery_audit BEFORE INSERT ON run_events
                 WHEN NEW.kind='recovery-confirmed-exited'
                 BEGIN SELECT RAISE(FAIL, 'synthetic audit failure'); END;",
            )
            .unwrap();

        assert_eq!(
            runtime.resolve_unknown_run(&run.id, true).unwrap_err().code,
            "storage.write-failed"
        );
        assert_eq!(runtime.get_run(&run.id).unwrap().state, RunState::Unknown);
        assert!(snapshot.join("README.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn fake_claude_success_persists_output_and_duplicate_request_reuses_run() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        std::fs::write(workspace.path().join("README.md"), "safe input").unwrap();
        let runtime = fake_runtime(root.path());
        let req = runtime_request(
            "00000000-0000-4000-8000-000000000101",
            workspace.path(),
            "Summarize the tree",
        );

        let (task, run) = runtime.start_read_only_task(req.clone(), |_| {}).unwrap();
        let completed = wait_for_terminal(&runtime, &run.id);
        let (same_task, same_run) = runtime.start_read_only_task(req, |_| {}).unwrap();

        assert_eq!(completed.outcome.as_deref(), Some("success"));
        assert_eq!(completed.summary.as_deref(), Some("fake result"));
        assert_eq!(runtime.list_run_events(&run.id, 0, 10).unwrap().len(), 1);
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("README.md")).unwrap(),
            "safe input"
        );
        assert_eq!(same_task.id, task.id);
        assert_eq!(same_run.id, run.id);
        assert_eq!(runtime.list_tasks().unwrap().len(), 1);
        assert_eq!(runtime.list_runs(None, 10).unwrap().items.len(), 1);
        assert!(!runtime
            .data_dir
            .join("run-snapshots")
            .join(&run.request_id)
            .exists());
    }

    #[cfg(unix)]
    #[test]
    fn fake_claude_nonzero_exit_is_persisted_as_failure() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let req = runtime_request(
            "00000000-0000-4000-8000-000000000103",
            workspace.path(),
            "fail this fake task",
        );
        let (_, run) = runtime.start_read_only_task(req, |_| {}).unwrap();

        let failed = wait_for_terminal(&runtime, &run.id);

        assert_eq!(failed.state, RunState::Crashed);
        assert_eq!(failed.outcome.as_deref(), Some("failed"));
        assert_eq!(failed.exit_code, Some(7));
        assert_eq!(runtime.get_task(&failed.task_id).unwrap().state, "failed");
    }

    #[cfg(unix)]
    #[test]
    fn fake_claude_stderr_is_classified_without_persisting_raw_content() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let req = runtime_request(
            "00000000-0000-4000-8000-000000000106",
            workspace.path(),
            "diag task",
        );
        let (_, run) = runtime.start_read_only_task(req, |_| {}).unwrap();

        let failed = wait_for_terminal(&runtime, &run.id);
        let events = runtime.list_run_events(&run.id, 0, 10).unwrap();
        let connection = rusqlite::Connection::open(runtime.data_dir.join("runs.sqlite3")).unwrap();
        let stored: String = connection
            .query_row(
                "SELECT coalesce(summary,'') || coalesce(error_code,'') FROM runs WHERE id=?1",
                [&run.id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            failed.error_code.as_deref(),
            Some("execution.cli-invalid-arguments")
        );
        assert!(!stored.contains("sk-ant-test-secret"));
        assert!(!stored.contains("/Users/private/person"));
        assert!(events.iter().all(|event| {
            !event.text.contains("sk-ant-test-secret")
                && !event.text.contains("/Users/private/person")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn orphaned_process_group_is_terminated_and_never_reported_as_success() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let req = runtime_request(
            "00000000-0000-4000-8000-000000000107",
            workspace.path(),
            "orphan process test",
        );
        let (_, run) = runtime.start_read_only_task(req, |_| {}).unwrap();

        let failed = wait_for_terminal(&runtime, &run.id);

        assert_eq!(failed.outcome.as_deref(), Some("child-process-remained"));
        assert_eq!(
            failed.error_code.as_deref(),
            Some("execution.child-process-remained")
        );
        let cleanup_deadline = Instant::now() + Duration::from_secs(1);
        while !runtime.active.lock().unwrap().is_empty() && Instant::now() < cleanup_deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(runtime.active.lock().unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn unknown_exit_retains_pid_and_snapshot_and_blocks_another_task() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let run = pending_running_run(&runtime, "00000000-0000-4000-8000-000000000108", 42_424);
        let snapshot = runtime.data_dir.join("run-snapshots").join(&run.request_id);
        std::fs::create_dir_all(&snapshot).unwrap();
        std::fs::write(snapshot.join("README.md"), "synthetic").unwrap();

        worker_for(&runtime, &run.id, snapshot.clone()).finalize(
            ProcessCompletion::ExitUnconfirmed {
                error_code: "run.stop-unconfirmed",
                pid: Some(42_424),
            },
        );

        let unknown = runtime.get_run(&run.id).unwrap();
        assert_eq!(unknown.state, RunState::Unknown);
        assert_eq!(unknown.process_id, Some(42_424));
        assert!(snapshot.join("README.md").exists());
        let blocked = runtime
            .start_read_only_task(
                runtime_request(
                    "00000000-0000-4000-8000-000000000109",
                    workspace.path(),
                    "do not retry",
                ),
                |_| {},
            )
            .unwrap_err();
        assert_eq!(blocked.code, "execution.process-state-unknown");
        assert_eq!(runtime.list_tasks().unwrap().len(), 1);

        let unchanged = runtime
            .store
            .lock()
            .unwrap()
            .finish(
                &run.id,
                RunFinish {
                    run_state: RunState::Finished,
                    outcome: "success",
                    task_state: "done",
                    exit_code: Some(0),
                    summary: Some("late success"),
                    error_code: None,
                    pid: None,
                },
            )
            .unwrap();
        assert_eq!(unchanged.state, RunState::Unknown);
    }

    #[cfg(unix)]
    #[test]
    fn failed_terminal_write_keeps_snapshot_and_converts_run_to_unknown() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let run = pending_running_run(&runtime, "00000000-0000-4000-8000-000000000110", 42_425);
        let snapshot = runtime.data_dir.join("run-snapshots").join(&run.request_id);
        std::fs::create_dir_all(&snapshot).unwrap();
        std::fs::write(snapshot.join("README.md"), "synthetic").unwrap();
        rusqlite::Connection::open(runtime.data_dir.join("runs.sqlite3"))
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_finished_run BEFORE UPDATE OF state ON runs
                 WHEN NEW.state IN ('finished','crashed')
                 BEGIN SELECT RAISE(FAIL, 'synthetic write failure'); END;",
            )
            .unwrap();

        worker_for(&runtime, &run.id, snapshot.clone()).finalize(
            ProcessCompletion::ExitedSuccessfully {
                exit_code: Some(0),
                summary: "synthetic result".into(),
            },
        );

        let unresolved = runtime.get_run(&run.id).unwrap();
        assert_eq!(unresolved.state, RunState::Unknown);
        assert_eq!(
            unresolved.error_code.as_deref(),
            Some("storage.write-failed")
        );
        assert_eq!(unresolved.process_id, Some(42_425));
        assert!(snapshot.join("README.md").exists());
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_cleanup_failure_does_not_rewrite_confirmed_result() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let run = pending_running_run(&runtime, "00000000-0000-4000-8000-000000000111", 42_426);
        let parent = runtime.data_dir.join("run-snapshots");
        std::fs::create_dir_all(&parent).unwrap();
        let snapshot = parent.join(&run.request_id);
        std::fs::write(&snapshot, "not a directory").unwrap();

        worker_for(&runtime, &run.id, snapshot.clone()).finalize(
            ProcessCompletion::ExitedSuccessfully {
                exit_code: Some(0),
                summary: "synthetic result".into(),
            },
        );

        assert_eq!(
            runtime.get_run(&run.id).unwrap().outcome.as_deref(),
            Some("success")
        );
        assert_eq!(
            std::fs::read_to_string(snapshot).unwrap(),
            "not a directory"
        );
    }

    #[cfg(unix)]
    #[test]
    fn fake_claude_timeout_is_persisted_as_failure() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let runtime = fake_runtime_with_timeout(root.path(), Duration::from_millis(100));
        let req = runtime_request(
            "00000000-0000-4000-8000-000000000104",
            workspace.path(),
            "slow task",
        );
        let (_, run) = runtime.start_read_only_task(req, |_| {}).unwrap();

        let timed_out = wait_for_terminal(&runtime, &run.id);

        assert_eq!(timed_out.outcome.as_deref(), Some("timeout"));
        assert_eq!(timed_out.error_code.as_deref(), Some("run.timeout"));
        assert_eq!(
            runtime.get_task(&timed_out.task_id).unwrap().state,
            "failed"
        );
    }

    #[cfg(unix)]
    #[test]
    fn fake_claude_invalid_protocol_is_persisted_without_raw_output() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let req = runtime_request(
            "00000000-0000-4000-8000-000000000105",
            workspace.path(),
            "protocol error",
        );
        let (_, run) = runtime.start_read_only_task(req, |_| {}).unwrap();

        let failed = wait_for_terminal(&runtime, &run.id);
        let events = runtime.list_run_events(&run.id, 0, 10).unwrap();

        assert_eq!(failed.outcome.as_deref(), Some("output-protocol-error"));
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].text,
            "无法解析 Claude Code 输出事件，原始内容已丢弃"
        );
    }

    #[cfg(unix)]
    #[test]
    fn fake_claude_stop_waits_for_child_process_tree_to_exit() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let req = runtime_request(
            "00000000-0000-4000-8000-000000000102",
            workspace.path(),
            "slow task",
        );
        let (_, run) = runtime.start_read_only_task(req, |_| {}).unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while runtime.get_run(&run.id).unwrap().state != RunState::Running {
            assert!(
                Instant::now() < deadline,
                "fake Claude process did not start"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let requested = runtime.stop_run(&run.id, |_| {}).unwrap();
        let stopped = wait_for_terminal(&runtime, &run.id);

        assert_eq!(requested.state, RunState::StopRequested);
        assert_eq!(stopped.outcome.as_deref(), Some("stopped"));
        let active_deadline = Instant::now() + Duration::from_secs(1);
        while !runtime.active.lock().unwrap().is_empty() {
            assert!(
                Instant::now() < active_deadline,
                "stale worker channel remained"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(runtime.active.lock().unwrap().is_empty());
    }

    #[test]
    fn task_and_run_are_created_atomically_and_request_id_is_unique() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("runs.sqlite3")).unwrap();
        let (task, run) = store
            .create_pending(
                &request("00000000-0000-4000-8000-000000000001"),
                "/tmp/workspace",
            )
            .unwrap();
        assert_eq!(task.current_run_id, run.id);
        assert_eq!(run.state, RunState::Pending);
        assert!(store.request_exists(&run.request_id).unwrap());
        assert!(store
            .create_pending(
                &request("00000000-0000-4000-8000-000000000001"),
                "/tmp/workspace"
            )
            .is_err());
    }

    #[test]
    fn events_have_monotonic_sequences_and_are_bounded() {
        let dir = tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("runs.sqlite3")).unwrap();
        let (_, run) = store
            .create_pending(
                &request("00000000-0000-4000-8000-000000000002"),
                "/tmp/workspace",
            )
            .unwrap();
        let first = store
            .append_event(&run.id, "text", "hello")
            .unwrap()
            .unwrap();
        assert_eq!(first.sequence, 1);
        let full_event = "x".repeat(64 * 1024);
        for _ in 0..31 {
            store.append_event(&run.id, "text", &full_event).unwrap();
        }
        let truncation = store
            .append_event(&run.id, "text", &full_event)
            .unwrap()
            .unwrap();
        assert_eq!(truncation.kind, "truncated");
        assert!(store
            .append_event(&run.id, "text", "ignored")
            .unwrap()
            .is_none());
    }

    #[test]
    fn request_validation_rejects_provider_and_oversized_prompt() {
        let mut req = request("00000000-0000-4000-8000-000000000005");
        req.provider_id = "codex".into();
        assert_eq!(
            validate_request(&req).unwrap_err().code,
            "execution.provider-not-supported"
        );
        req.provider_id = "claude".into();
        req.prompt = "x".repeat(MAX_PROMPT_BYTES + 1);
        assert_eq!(
            validate_request(&req).unwrap_err().code,
            "execution.invalid-request"
        );
    }

    #[test]
    fn corrupt_database_is_preserved_and_reports_a_recoverable_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        let original = b"not a sqlite database";
        std::fs::write(&path, original).unwrap();
        let error = match Store::open(&path) {
            Ok(_) => panic!("corrupt database unexpectedly opened"),
            Err(error) => error,
        };
        assert_eq!(error.code, "storage.read-failed");
        assert_eq!(std::fs::read(path).unwrap(), original);
    }

    #[test]
    fn stream_parser_accepts_a_valid_final_record_without_trailing_newline() {
        let (tx, rx) = mpsc::sync_channel(2);
        read_stream(
            std::io::Cursor::new(br#"{"type":"result","result":"done"}"#.to_vec()),
            tx,
        );
        assert_eq!(rx.recv().unwrap(), ClaudeOutput::Final("done".into()));
    }

    #[test]
    fn active_runs_recover_as_unknown_without_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("runs.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let (task, run) = store
            .create_pending(
                &request("00000000-0000-4000-8000-000000000003"),
                "/tmp/workspace",
            )
            .unwrap();
        drop(store);
        let mut store = Store::open(&path).unwrap();
        store.recover_active().unwrap();
        assert_eq!(store.get_run(&run.id).unwrap().state, RunState::Unknown);
        assert_eq!(store.get_task(&task.id).unwrap().state, "unknown");
        assert!(store.has_uncertain_process().unwrap());
        assert_eq!(
            store
                .create_pending(
                    &request("00000000-0000-4000-8000-000000000006"),
                    "/tmp/workspace",
                )
                .unwrap_err()
                .code,
            "execution.process-state-unknown"
        );
    }

    #[test]
    fn invalid_request_ids_and_missing_consent_are_rejected() {
        let mut req = request("not-a-uuid");
        assert!(validate_request(&req).is_err());
        req = request("00000000-0000-4000-8000-000000000004");
        req.consent = false;
        assert_eq!(
            validate_request(&req).unwrap_err().code,
            "execution.consent-required"
        );
    }

    #[test]
    fn catalog_crud_remains_available_while_execution_is_fail_closed() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let source = std::fs::canonicalize(source).unwrap();
        let runtime =
            ExecutionRuntime::open(dir.path().join("config"), dir.path().join("data")).unwrap();
        let readiness = runtime.readiness();
        assert!(!readiness.ready);
        assert_eq!(
            readiness.reason_code.as_deref(),
            Some("execution.managed-policy-unverified")
        );

        let workspace = runtime
            .create_workspace(CreateWorkspaceInput {
                name: "Local".into(),
                path: source.to_string_lossy().into_owned(),
            })
            .unwrap();
        let agent = runtime
            .create_agent(CreateAgentInput {
                name: "Reviewer".into(),
                role: "只读分析".into(),
                provider_id: "claude".into(),
                description: String::new(),
                instructions: String::new(),
                permissions: Default::default(),
            })
            .unwrap();
        let team = runtime
            .create_team(CreateTeamInput {
                name: "Review team".into(),
                supervisor_id: agent.id.clone(),
                workspace_id: workspace.id,
            })
            .unwrap();
        assert_eq!(
            runtime.get_team(&team.id).unwrap().members[0].agent_id,
            agent.id
        );
        assert!(!runtime.readiness().ready);

        let mut appearance = runtime.appearance_settings().unwrap();
        appearance.theme = "dark".into();
        runtime
            .update_appearance_settings(appearance.clone())
            .unwrap();
        assert_eq!(runtime.appearance_settings().unwrap(), appearance);
        assert_eq!(runtime.list_agents().unwrap().len(), 1);
    }
}
