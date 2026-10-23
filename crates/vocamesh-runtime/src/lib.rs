//! Local single-run execution lifecycle and SQLite history.

mod catalog_store;
mod provider_policy;
mod snapshot;
mod store;

use crate::{
    snapshot::{create_filtered_snapshot, validate_workspace},
    store::{RunFinish, Store},
};
use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::{Child, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
#[cfg(not(windows))]
use vocamesh_agent::AgentRuntime;
use vocamesh_agent::{
    execution::ExecutionEnvironment,
    runner::{parse_provider_output, runner_for, ProviderOutput, ProviderRunner},
};
use vocamesh_core::{
    AddTeamMemberInput, AgentRole, AppearanceSettings, AuthenticationState, CatalogStatus,
    CreateAgentInput, CreateTeamInput, CreateWorkspaceInput, ExecutionBlocker, ExecutionCheck,
    ExecutionCheckId, ExecutionCheckStatus, ExecutionPolicyStatus, ExecutionReadiness,
    ExecutionRequest, LocalTool, ProcessGroupStatus, Provider, RecoveryInspection,
    RecoveryResolution, RecoveryStatus, RunEvent, RunRecord, RunState, ServiceError,
    SnapshotStatus, TaskRecord, TeamRecord, ToolDiscoveryStatus, ToolVersionStatus,
    UnknownRunSummary, UpdateAgentInput, UpdateTeamInput, UpdateWorkspaceScopesInput,
    WorkspaceRecord,
};

#[cfg(windows)]
const ERR_CLI: ServiceError = ServiceError::new(
    "execution.cli-not-ready",
    "Claude Code CLI 未通过安全版本检查",
);
const ERR_AUTH: ServiceError = ServiceError::new(
    "execution.auth-required",
    "Claude Code 尚未确认已登录，请先在 CLI 中完成登录",
);
const ERR_AUTH_UNKNOWN: ServiceError = ServiceError::new(
    "execution.auth-unknown",
    "无法确认 Provider 登录状态；请检查 CLI 后重新验证",
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
const ERR_CODEX_POLICY_UNVERIFIED: ServiceError = ServiceError::new(
    "execution.codex-safety-unverified",
    "Codex 的配置、凭据、文件、进程和网络边界尚未证明；真实执行已禁用",
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
    #[cfg(test)]
    TestCodexSafe,
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
        self.readiness_for(Provider::Claude)
    }

    pub fn readiness_for(&self, provider: Provider) -> ExecutionReadiness {
        #[cfg(not(windows))]
        {
            let result = AgentRuntime::new(self.config_dir.clone()).verify_tool(provider.id());
            self.readiness_from_tool_result(provider, result)
        }
        #[cfg(windows)]
        {
            self.readiness_from_tool_result(provider, Err(ERR_CLI))
        }
    }

    /// Build diagnostics from an already completed discovery or verification pass.
    /// A regular tool scan passes its unverified auth state; this method never probes a CLI.
    pub fn readiness_for_tool(&self, tool: &LocalTool) -> ExecutionReadiness {
        let provider = tool.id.parse::<Provider>().unwrap_or(Provider::Opencode);
        self.readiness_from_tool_result(provider, Ok(tool.clone()))
    }

    fn readiness_from_tool_result(
        &self,
        provider: Provider,
        tool_result: Result<LocalTool, ServiceError>,
    ) -> ExecutionReadiness {
        let mut checks = Vec::with_capacity(9);
        let mut blockers = Vec::with_capacity(8);
        let (active, uncertain) = match self.store.lock() {
            Ok(store) => (
                store.has_active().map_err(|error| error.code),
                store.has_uncertain_process().map_err(|error| error.code),
            ),
            Err(_) => (Err("storage.read-failed"), Err("storage.read-failed")),
        };
        let storage_ok = active.is_ok() && uncertain.is_ok();
        append_execution_check(
            &mut checks,
            &mut blockers,
            ExecutionCheckId::LocalStorage,
            if storage_ok {
                ExecutionCheckStatus::Passed
            } else {
                ExecutionCheckStatus::Failed
            },
            (!storage_ok).then_some("storage.read-failed"),
        );
        append_execution_check(
            &mut checks,
            &mut blockers,
            ExecutionCheckId::ActiveRun,
            match active {
                Ok(false) => ExecutionCheckStatus::Passed,
                Ok(true) => ExecutionCheckStatus::Blocked,
                Err(_) => ExecutionCheckStatus::Unknown,
            },
            match active {
                Ok(false) => None,
                Ok(true) => Some("execution.already-running"),
                Err(code) => Some(code),
            },
        );
        append_execution_check(
            &mut checks,
            &mut blockers,
            ExecutionCheckId::ProcessState,
            match uncertain {
                Ok(false) => ExecutionCheckStatus::Passed,
                Ok(true) => ExecutionCheckStatus::Blocked,
                Err(_) => ExecutionCheckStatus::Unknown,
            },
            match uncertain {
                Ok(false) => None,
                Ok(true) => Some("execution.process-state-unknown"),
                Err(code) => Some(code),
            },
        );

        #[cfg(windows)]
        let platform_status = ExecutionCheckStatus::Unsupported;
        #[cfg(not(windows))]
        let platform_status = ExecutionCheckStatus::Passed;
        append_execution_check(
            &mut checks,
            &mut blockers,
            ExecutionCheckId::Platform,
            platform_status,
            if cfg!(windows) {
                Some("execution.platform-unsupported")
            } else {
                None
            },
        );

        let (version, auth_state) = match tool_result {
            Ok(tool) => {
                let discovery_reason = match tool.discovery_status {
                    ToolDiscoveryStatus::Found => None,
                    ToolDiscoveryStatus::Missing => Some("execution.cli-not-found"),
                    ToolDiscoveryStatus::PathInvalid => Some("execution.cli-discovery-failed"),
                    ToolDiscoveryStatus::ProbeFailed => Some(
                        tool.reason_code
                            .as_deref()
                            .unwrap_or("execution.cli-probe-failed"),
                    ),
                };
                append_execution_check(
                    &mut checks,
                    &mut blockers,
                    ExecutionCheckId::CliDiscovery,
                    if tool.discovery_status == ToolDiscoveryStatus::Found {
                        ExecutionCheckStatus::Passed
                    } else if tool.discovery_status == ToolDiscoveryStatus::ProbeFailed {
                        ExecutionCheckStatus::Failed
                    } else {
                        ExecutionCheckStatus::Blocked
                    },
                    discovery_reason,
                );
                let version_reason = match tool.version_status {
                    ToolVersionStatus::Known => None,
                    ToolVersionStatus::Incompatible => Some("execution.cli-incompatible"),
                    ToolVersionStatus::Unknown => Some(
                        tool.reason_code
                            .as_deref()
                            .filter(|code| code.starts_with("tool."))
                            .unwrap_or("execution.cli-version-unknown"),
                    ),
                };
                append_execution_check(
                    &mut checks,
                    &mut blockers,
                    ExecutionCheckId::CliVersion,
                    match tool.version_status {
                        ToolVersionStatus::Known => ExecutionCheckStatus::Passed,
                        ToolVersionStatus::Incompatible => ExecutionCheckStatus::Blocked,
                        ToolVersionStatus::Unknown => ExecutionCheckStatus::Unknown,
                    },
                    version_reason,
                );
                let auth_reason = match tool.auth_state {
                    AuthenticationState::Authenticated => None,
                    AuthenticationState::Unauthenticated => Some("execution.auth-required"),
                    AuthenticationState::Unsupported => Some("execution.auth-unsupported"),
                    AuthenticationState::Unknown => Some(
                        tool.reason_code
                            .as_deref()
                            .filter(|code| code.starts_with("execution.auth-"))
                            .unwrap_or("execution.auth-unknown"),
                    ),
                };
                append_execution_check(
                    &mut checks,
                    &mut blockers,
                    ExecutionCheckId::Authentication,
                    match tool.auth_state {
                        AuthenticationState::Authenticated => ExecutionCheckStatus::Passed,
                        AuthenticationState::Unauthenticated => ExecutionCheckStatus::Blocked,
                        AuthenticationState::Unsupported => ExecutionCheckStatus::Unsupported,
                        AuthenticationState::Unknown => ExecutionCheckStatus::Unknown,
                    },
                    auth_reason,
                );
                (
                    (tool.version_status == ToolVersionStatus::Known).then_some(tool.version),
                    tool.auth_state,
                )
            }
            Err(error) => {
                let discovery_reason = match error.code {
                    "tool.not-found" => "execution.cli-not-found",
                    "settings.read-failed" => "settings.read-failed",
                    _ => "execution.cli-discovery-failed",
                };
                append_execution_check(
                    &mut checks,
                    &mut blockers,
                    ExecutionCheckId::CliDiscovery,
                    ExecutionCheckStatus::Failed,
                    Some(discovery_reason),
                );
                append_execution_check(
                    &mut checks,
                    &mut blockers,
                    ExecutionCheckId::CliVersion,
                    ExecutionCheckStatus::Unknown,
                    Some(discovery_reason),
                );
                append_execution_check(
                    &mut checks,
                    &mut blockers,
                    ExecutionCheckId::Authentication,
                    ExecutionCheckStatus::Unknown,
                    Some("execution.auth-unknown"),
                );
                (None, AuthenticationState::Unknown)
            }
        };

        let policy_error = if provider == Provider::Codex && self.test_codex_policy_allowed() {
            None
        } else {
            provider_policy::evaluate(provider, self.managed_policy_is_verified()).err()
        };
        let policy_status = match policy_error {
            Some(error) => {
                append_execution_check(
                    &mut checks,
                    &mut blockers,
                    ExecutionCheckId::ManagedPolicy,
                    ExecutionCheckStatus::Blocked,
                    Some(error.code),
                );
                ExecutionPolicyStatus::Unverified
            }
            None => {
                append_execution_check(
                    &mut checks,
                    &mut blockers,
                    ExecutionCheckId::ManagedPolicy,
                    ExecutionCheckStatus::Passed,
                    None,
                );
                ExecutionPolicyStatus::Verified
            }
        };
        let ready = blockers.is_empty();
        let reason_code = if ready {
            None
        } else {
            blockers
                .iter()
                .find(|blocker| blocker.check == ExecutionCheckId::ManagedPolicy)
                .or_else(|| blockers.first())
                .map(|blocker| blocker.reason_code.clone())
        };
        let message = readiness_message(reason_code.as_deref());
        checks.push(ExecutionCheck {
            id: ExecutionCheckId::TaskStart,
            status: if ready {
                ExecutionCheckStatus::Passed
            } else {
                ExecutionCheckStatus::Blocked
            },
            reason_code: reason_code.clone(),
        });
        ExecutionReadiness {
            ready,
            version,
            auth_state,
            message: message.to_owned(),
            reason_code,
            policy_status,
            checks,
            blockers,
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
        let provider = request.provider_id.parse::<Provider>().map_err(|_| {
            ServiceError::new("execution.provider-not-supported", "Provider 当前不可执行")
        })?;
        {
            let store = self.store.lock().map_err(|_| storage_read())?;
            if let Some(existing) = store.has_request(&request.request_id)? {
                let (task, run) = existing;
                let requested_workspace = std::fs::canonicalize(&request.workspace_path)
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| request.workspace_path.clone());
                if task.provider_id != request.provider_id
                    || task.title != request.title
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
        self.ensure_provider_policy(provider)?;
        let environment = ExecutionEnvironment::current_for(provider);
        let executable = AgentRuntime::new(self.config_dir.clone())
            .executable_path(provider)
            .map_err(|_| {
                ServiceError::new("execution.cli-not-found", "未找到受支持的 Provider CLI")
            })?;
        let runner = runner_for(provider, executable, environment)?;
        match runner.probe()? {
            AuthenticationState::Authenticated => {}
            AuthenticationState::Unauthenticated => return Err(ERR_AUTH),
            AuthenticationState::Unsupported if self.test_codex_policy_allowed() => {}
            AuthenticationState::Unknown | AuthenticationState::Unsupported => {
                return Err(ERR_AUTH_UNKNOWN)
            }
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
            runner,
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

    fn ensure_provider_policy(&self, provider: Provider) -> Result<(), ServiceError> {
        if self.test_codex_policy_allowed() && provider == Provider::Codex {
            return Ok(());
        }
        provider_policy::evaluate(provider, self.managed_policy_is_verified())
    }

    fn managed_policy_is_verified(&self) -> bool {
        match self.managed_policy {
            ManagedPolicyStatus::Unverified => false,
            #[cfg(test)]
            ManagedPolicyStatus::TestVerifiedSafe => true,
            #[cfg(test)]
            ManagedPolicyStatus::TestCodexSafe => false,
        }
    }

    fn test_codex_policy_allowed(&self) -> bool {
        #[cfg(test)]
        {
            self.managed_policy == ManagedPolicyStatus::TestCodexSafe
        }
        #[cfg(not(test))]
        {
            false
        }
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
    runner: Arc<dyn ProviderRunner>,
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
        if let Err(error) = self.runtime.ensure_provider_policy(self.runner.provider()) {
            return failure(error.code);
        }
        match self.runner.probe() {
            Ok(AuthenticationState::Authenticated) => {}
            Ok(_) if self.runtime.test_codex_policy_allowed() => {}
            Ok(AuthenticationState::Unauthenticated) => return failure("execution.auth-required"),
            Ok(_) | Err(_) => return failure("execution.auth-unknown"),
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
        let mut command = self
            .runner
            .launch_command(&self.prompt, &self.snapshot_path);
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
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
        if self.runner.prompt_on_stdin() {
            let Some(mut stdin) = child.stdin.take() else {
                return if terminate_process_tree(child, pid) {
                    failure("execution.stdin-unavailable")
                } else {
                    ProcessCompletion::ExitUnconfirmed {
                        error_code: "run.stop-unconfirmed",
                        pid: Some(pid),
                    }
                };
            };
            if stdin.write_all(self.prompt.as_bytes()).is_err() {
                return if terminate_process_tree(child, pid) {
                    failure("execution.stdin-write-failed")
                } else {
                    ProcessCompletion::ExitUnconfirmed {
                        error_code: "run.stop-unconfirmed",
                        pid: Some(pid),
                    }
                };
            }
        }
        let (output_tx, output_rx) = mpsc::sync_channel::<ProviderOutput>(128);
        let stdout_tx = output_tx.clone();
        let provider = self.runner.provider();
        thread::spawn(move || read_stream(stdout, stdout_tx, provider));
        let (stderr_tx, stderr_rx) = mpsc::channel();
        let runner = Arc::clone(&self.runner);
        thread::spawn(move || {
            let bounded = vocamesh_agent::execution::read_bounded_stderr(stderr);
            let _ = stderr_tx.send(runner.classify_failure(&bounded));
        });
        drop(output_tx);

        let start = Instant::now();
        let mut stop_requested = false;
        let mut timed_out = false;
        let mut output_state = OutputState::default();
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
                &mut output_state,
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
                    if store_output(
                        &self.runtime,
                        &self.run_id,
                        output,
                        &self.notify,
                        &mut output_state,
                    )
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
        let stderr_error = stderr_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or("execution.cli-failed");
        let final_summary = output_state.summary;
        if stop_requested {
            ProcessCompletion::StoppedAndExited { exit_code: code }
        } else if timed_out {
            ProcessCompletion::TimedOutAndExited { exit_code: code }
        } else if output_state.provider_failed {
            ProcessCompletion::ExitedWithFailure {
                outcome: "provider-failed",
                exit_code: code,
                summary: final_summary,
                error_code: "execution.provider-failed",
            }
        } else if output_state.protocol_error {
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
                error_code: stderr_error,
            }
        } else if !output_state.protocol_complete || final_summary.is_none() {
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
    rx: &mpsc::Receiver<ProviderOutput>,
    notify: &Arc<dyn Fn(RuntimeNotification) + Send + Sync>,
    state: &mut OutputState,
) -> Result<(), &'static str> {
    while let Ok(output) = rx.try_recv() {
        store_output(runtime, run_id, output, notify, state)?;
    }
    Ok(())
}

#[derive(Default)]
struct OutputState {
    summary: Option<String>,
    protocol_complete: bool,
    provider_failed: bool,
    protocol_error: bool,
}

fn store_output(
    runtime: &ExecutionRuntime,
    run_id: &str,
    output: ProviderOutput,
    notify: &Arc<dyn Fn(RuntimeNotification) + Send + Sync>,
    state: &mut OutputState,
) -> Result<(), &'static str> {
    let (kind, text) = match output {
        ProviderOutput::TextDelta(text) => ("text", text),
        ProviderOutput::Final(text) => {
            state.summary = Some(text.clone());
            ("final", text)
        }
        ProviderOutput::FinalAndComplete(text) => {
            state.summary = Some(text.clone());
            state.protocol_complete = true;
            ("final", text)
        }
        ProviderOutput::ToolProgress(text) => ("progress", text.to_owned()),
        ProviderOutput::ProtocolComplete => {
            state.protocol_complete = true;
            return Ok(());
        }
        ProviderOutput::ProtocolFailed => {
            state.provider_failed = true;
            (
                "error",
                "Provider 报告任务失败，原始错误内容已丢弃".to_owned(),
            )
        }
        ProviderOutput::ProtocolError => {
            state.protocol_error = true;
            (
                "protocol-error",
                "无法解析 Provider 输出事件，原始内容已丢弃".to_owned(),
            )
        }
        ProviderOutput::Ignore => return Ok(()),
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
    Ok(())
}

fn read_stream<R: std::io::Read>(
    mut reader: R,
    tx: mpsc::SyncSender<ProviderOutput>,
    provider: Provider,
) {
    let mut buffer = [0u8; 8192];
    let mut line = Vec::with_capacity(1024);
    let mut too_long = false;
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => break,
            Err(_) => {
                let _ = tx.send(ProviderOutput::ProtocolError);
                return;
            }
            Ok(count) => count,
        };
        for byte in &buffer[..count] {
            if *byte == b'\n' {
                let event = if too_long {
                    ProviderOutput::ProtocolError
                } else {
                    parse_provider_output(provider, &line)
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
            ProviderOutput::ProtocolError
        } else {
            parse_provider_output(provider, &line)
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
    if !matches!(
        request.provider_id.parse::<Provider>(),
        Ok(Provider::Claude | Provider::Codex)
    ) {
        return Err(ServiceError::new(
            "execution.provider-not-supported",
            "当前只支持 Claude Code 和 Codex 的受控执行适配器",
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

fn append_execution_check(
    checks: &mut Vec<ExecutionCheck>,
    blockers: &mut Vec<ExecutionBlocker>,
    id: ExecutionCheckId,
    status: ExecutionCheckStatus,
    reason_code: Option<&str>,
) {
    let reason_code = reason_code.map(str::to_owned);
    checks.push(ExecutionCheck {
        id,
        status,
        reason_code: reason_code.clone(),
    });
    if status != ExecutionCheckStatus::Passed {
        blockers.push(ExecutionBlocker {
            check: id,
            reason_code: reason_code.unwrap_or_else(|| "execution.check-unknown".into()),
        });
    }
}

fn readiness_message(reason_code: Option<&str>) -> &'static str {
    match reason_code {
        None => "Provider 已认证，所有 Runtime 硬门禁均已通过",
        Some("storage.read-failed") => "本地任务历史无法读取，已阻止执行",
        Some("execution.process-state-unknown") => {
            "先前进程状态无法确认；为避免重复启动，真实执行已暂停"
        }
        Some("execution.already-running") => "本机 Run 尚未完成状态核对；真实执行已暂停",
        Some("execution.platform-unsupported") => {
            "当前平台的进程安全策略未通过验证，真实执行暂不可用"
        }
        Some("execution.cli-not-found") => "未找到受支持的 Claude Code CLI",
        Some("execution.cli-discovery-failed") => "Claude Code CLI 发现检查失败",
        Some("execution.cli-version-unknown") => "Claude Code CLI 版本输出无法确认",
        Some("execution.cli-incompatible") => "Claude Code CLI 版本不兼容",
        Some("execution.auth-required") => "Claude Code CLI 尚未登录，请先完成登录",
        Some("execution.auth-not-verified") => "尚未执行 Claude Code 认证探测",
        Some("execution.auth-output-unrecognized") => {
            "Claude Code 认证输出无法识别，登录状态保持未知"
        }
        Some("execution.auth-probe-timeout") => "Claude Code 认证探测超时，登录状态保持未知",
        Some("execution.auth-probe-failed") => "Claude Code 认证探测失败，登录状态保持未知",
        Some("execution.auth-cli-incompatible") => "当前 Claude Code 版本不支持受控认证探测",
        Some("execution.auth-unknown") => "无法确认 Claude Code 登录状态，已阻止执行",
        Some("execution.auth-unsupported") => "当前 Provider 不支持安全认证探测",
        Some("execution.managed-policy-unverified") => ERR_POLICY_UNVERIFIED.message,
        Some("execution.codex-safety-unverified") => ERR_CODEX_POLICY_UNVERIFIED.message,
        Some("execution.cli-probe-failed") => "Claude Code CLI 预检失败",
        Some(_) => "Runtime 检查未通过，真实执行已阻止",
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
    use std::io::Read;
    use std::process::Command;
    use tempfile::tempdir;
    use vocamesh_core::ToolState;

    #[cfg(unix)]
    static FAKE_CLI_TEST_GATE: Mutex<()> = Mutex::new(());

    #[cfg(unix)]
    fn fake_cli_test_guard() -> std::sync::MutexGuard<'static, ()> {
        FAKE_CLI_TEST_GATE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct FinalizeOnlyRunner;

    impl ProviderRunner for FinalizeOnlyRunner {
        fn provider(&self) -> Provider {
            Provider::Claude
        }

        fn executable(&self) -> &std::path::Path {
            std::path::Path::new("/unused")
        }

        fn version(&self) -> Option<String> {
            None
        }

        fn probe(&self) -> Result<AuthenticationState, ServiceError> {
            Ok(AuthenticationState::Unknown)
        }

        fn launch_command(&self, _prompt: &str, _working_directory: &std::path::Path) -> Command {
            Command::new("/unused")
        }

        fn prompt_on_stdin(&self) -> bool {
            false
        }

        fn classify_failure(&self, _stderr: &[u8]) -> &'static str {
            "execution.cli-failed"
        }
    }

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

    fn preflight_tool(auth_state: AuthenticationState, reason_code: Option<&str>) -> LocalTool {
        let mut tool = LocalTool::unavailable(
            Provider::Claude,
            ToolState::Undetectable,
            "synthetic diagnostic fixture",
        );
        tool.discovery_status = ToolDiscoveryStatus::Found;
        tool.version_status = ToolVersionStatus::Known;
        tool.version = "2.1.296".into();
        tool.auth_state = auth_state;
        tool.reason_code = reason_code.map(str::to_owned);
        tool
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
        let executable = root.join("claude");
        std::fs::write(
            &executable,
            r##"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf '2.1.296 (Claude Code)\n'
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "status" ]; then
  printf '{"loggedIn":true,"authMethod":"oauth_token"}\n'
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
    fn fake_codex_runtime(root: &std::path::Path) -> ExecutionRuntime {
        fake_codex_runtime_with_timeout(root, RUN_TIMEOUT)
    }

    #[cfg(unix)]
    fn fake_codex_runtime_with_timeout(
        root: &std::path::Path,
        run_timeout: Duration,
    ) -> ExecutionRuntime {
        use std::os::unix::fs::PermissionsExt;

        let config = root.join("config");
        let data = root.join("data");
        std::fs::create_dir_all(&config).unwrap();
        let executable = root.join("codex");
        std::fs::write(
            &executable,
            r##"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf 'codex-cli 0.162.1\n'
  exit 0
fi
if [ "$1" = "exec" ] && [ "$2" = "--help" ]; then
  printf 'codex exec --json --sandbox read-only --ignore-user-config --ignore-rules --ephemeral --cd\n'
  exit 0
fi
if [ "$1" = "exec" ]; then
  prompt=$(cat)
  case "$prompt" in
    *slow*)
      printf '{"type":"turn.started"}\n'
      sleep 30
      ;;
    *failed*)
      printf '{"type":"turn.failed","error":{"message":"synthetic failure"}}\n'
      exit 0
      ;;
    *missing-completion*)
      printf '{"type":"item.completed","item":{"type":"agent_message","text":"candidate only"}}\n'
      exit 0
      ;;
  esac
  printf '{"type":"thread.started","thread_id":"fixture-thread"}\n'
  printf '{"type":"turn.started"}\n'
  printf '{"type":"item.completed","item":{"type":"agent_message","text":"VOCAMESH synthetic marker"}}\n'
  printf '{"type":"turn.completed"}\n'
  exit 0
fi
exit 2
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
                "manualPaths": { "codex": executable },
                "defaultToolId": null,
            })
            .to_string(),
        )
        .unwrap();
        ExecutionRuntime::open_with_policy(
            config,
            data,
            run_timeout,
            ManagedPolicyStatus::TestCodexSafe,
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

    struct ChunkedReader {
        bytes: std::io::Cursor<Vec<u8>>,
        max_chunk: usize,
    }

    impl ChunkedReader {
        fn new(bytes: Vec<u8>, max_chunk: usize) -> Self {
            Self {
                bytes: std::io::Cursor::new(bytes),
                max_chunk,
            }
        }
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = buffer.len().min(self.max_chunk);
            self.bytes.read(&mut buffer[..count])
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
            runner: Arc::new(FinalizeOnlyRunner),
            snapshot_path,
            stop_rx,
            notify: Arc::new(|_| {}),
        }
    }

    #[test]
    fn managed_policy_uncertainty_blocks_readiness_and_task_start() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let mut runtime = fake_runtime(root.path());
        runtime.managed_policy = ManagedPolicyStatus::Unverified;

        let readiness = runtime.readiness();
        assert!(!readiness.ready);
        assert_eq!(
            readiness.reason_code.as_deref(),
            Some("execution.managed-policy-unverified")
        );
        assert_eq!(readiness.auth_state, AuthenticationState::Authenticated);
        assert_eq!(readiness.policy_status, ExecutionPolicyStatus::Unverified);
        assert!(readiness.checks.iter().any(|check| {
            check.id == ExecutionCheckId::Authentication
                && check.status == ExecutionCheckStatus::Passed
        }));
        assert!(readiness.blockers.iter().any(|blocker| {
            blocker.check == ExecutionCheckId::ManagedPolicy
                && blocker.reason_code == "execution.managed-policy-unverified"
        }));
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
    fn readiness_collects_auth_and_policy_blockers_without_short_circuiting() {
        let root = tempdir().unwrap();
        let mut runtime = fake_runtime(root.path());
        runtime.managed_policy = ManagedPolicyStatus::Unverified;

        let unauthenticated = runtime.readiness_for_tool(&preflight_tool(
            AuthenticationState::Unauthenticated,
            Some("execution.auth-required"),
        ));
        assert!(!unauthenticated.ready);
        assert_eq!(
            unauthenticated.auth_state,
            AuthenticationState::Unauthenticated
        );
        assert!(unauthenticated.blockers.iter().any(|blocker| {
            blocker.check == ExecutionCheckId::Authentication
                && blocker.reason_code == "execution.auth-required"
        }));
        assert!(unauthenticated.blockers.iter().any(|blocker| {
            blocker.check == ExecutionCheckId::ManagedPolicy
                && blocker.reason_code == "execution.managed-policy-unverified"
        }));

        let timeout = runtime.readiness_for_tool(&preflight_tool(
            AuthenticationState::Unknown,
            Some("execution.auth-probe-timeout"),
        ));
        let auth_check = timeout
            .checks
            .iter()
            .find(|check| check.id == ExecutionCheckId::Authentication)
            .unwrap();
        assert_eq!(auth_check.status, ExecutionCheckStatus::Unknown);
        assert_eq!(
            auth_check.reason_code.as_deref(),
            Some("execution.auth-probe-timeout")
        );
        assert!(timeout
            .blockers
            .iter()
            .any(|blocker| { blocker.check == ExecutionCheckId::ManagedPolicy }));
    }

    #[cfg(unix)]
    #[test]
    fn readiness_reports_cli_missing_and_policy_as_separate_blockers() {
        let root = tempdir().unwrap();
        let mut runtime = fake_runtime(root.path());
        runtime.managed_policy = ManagedPolicyStatus::Unverified;
        let missing = LocalTool::unavailable(
            Provider::Claude,
            ToolState::Missing,
            "synthetic missing CLI",
        );

        let readiness = runtime.readiness_for_tool(&missing);
        assert!(readiness.blockers.iter().any(|blocker| {
            blocker.check == ExecutionCheckId::CliDiscovery
                && blocker.reason_code == "execution.cli-not-found"
        }));
        assert!(readiness.blockers.iter().any(|blocker| {
            blocker.check == ExecutionCheckId::ManagedPolicy
                && blocker.reason_code == "execution.managed-policy-unverified"
        }));
    }

    #[cfg(unix)]
    #[test]
    fn readiness_passes_only_with_test_policy_and_all_preflight_checks() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());

        let readiness = runtime.readiness();
        assert!(readiness.ready);
        assert_eq!(readiness.policy_status, ExecutionPolicyStatus::Verified);
        assert!(readiness.blockers.is_empty());
        assert!(readiness.checks.iter().all(|check| {
            check.id == ExecutionCheckId::TaskStart || check.status == ExecutionCheckStatus::Passed
        }));
    }

    #[cfg(unix)]
    #[test]
    fn readiness_keeps_unknown_auth_blocking_when_test_policy_is_verified() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());

        let readiness = runtime.readiness_for_tool(&preflight_tool(
            AuthenticationState::Unknown,
            Some("execution.auth-probe-timeout"),
        ));
        assert!(!readiness.ready);
        assert_eq!(readiness.policy_status, ExecutionPolicyStatus::Verified);
        assert_eq!(readiness.auth_state, AuthenticationState::Unknown);
        assert!(readiness.blockers.iter().any(|blocker| {
            blocker.check == ExecutionCheckId::Authentication
                && blocker.reason_code == "execution.auth-probe-timeout"
        }));
        assert!(!readiness
            .blockers
            .iter()
            .any(|blocker| blocker.check == ExecutionCheckId::ManagedPolicy));
    }

    #[cfg(unix)]
    #[test]
    fn readiness_keeps_unknown_run_blocking_when_authentication_passes() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        let (_run, _snapshot) = unknown_run_with_group(
            &runtime,
            "00000000-0000-4000-8000-000000000196",
            42_496,
            None,
        );

        let readiness =
            runtime.readiness_for_tool(&preflight_tool(AuthenticationState::Authenticated, None));
        assert!(!readiness.ready);
        assert_eq!(readiness.auth_state, AuthenticationState::Authenticated);
        assert!(readiness.blockers.iter().any(|blocker| {
            blocker.check == ExecutionCheckId::ProcessState
                && blocker.reason_code == "execution.process-state-unknown"
        }));
    }

    #[cfg(unix)]
    #[test]
    fn readiness_keeps_active_run_blocking_when_authentication_passes() {
        let root = tempdir().unwrap();
        let runtime = fake_runtime(root.path());
        pending_running_run(&runtime, "00000000-0000-4000-8000-000000000198", 42_498);

        let readiness =
            runtime.readiness_for_tool(&preflight_tool(AuthenticationState::Authenticated, None));
        assert!(!readiness.ready);
        assert_eq!(readiness.auth_state, AuthenticationState::Authenticated);
        assert!(readiness.blockers.iter().any(|blocker| {
            blocker.check == ExecutionCheckId::ActiveRun
                && blocker.reason_code == "execution.already-running"
        }));
    }

    #[cfg(unix)]
    #[test]
    fn task_start_rechecks_policy_after_a_ready_diagnostic() {
        let root = tempdir().unwrap();
        let workspace = tempdir().unwrap();
        let mut runtime = fake_runtime(root.path());
        assert!(runtime.readiness().ready);
        runtime.managed_policy = ManagedPolicyStatus::Unverified;

        let error = runtime
            .start_read_only_task(
                runtime_request(
                    "00000000-0000-4000-8000-000000000197",
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
        let _fake_cli_guard = fake_cli_test_guard();
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
        let (same_task, same_run) = runtime.start_read_only_task(req.clone(), |_| {}).unwrap();
        let mut different_provider = req;
        different_provider.provider_id = "codex".into();
        assert_eq!(
            runtime
                .start_read_only_task(different_provider, |_| {})
                .unwrap_err()
                .code,
            "execution.request-duplicate"
        );

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
        let _fake_cli_guard = fake_cli_test_guard();
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
        let _fake_cli_guard = fake_cli_test_guard();
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
        let _fake_cli_guard = fake_cli_test_guard();
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
        let _fake_cli_guard = fake_cli_test_guard();
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
        let _fake_cli_guard = fake_cli_test_guard();
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
        assert_eq!(events[0].text, "无法解析 Provider 输出事件，原始内容已丢弃");
    }

    #[cfg(unix)]
    #[test]
    fn fake_claude_stop_waits_for_child_process_tree_to_exit() {
        let _fake_cli_guard = fake_cli_test_guard();
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
    fn request_validation_accepts_codex_adapter_and_rejects_unsupported_provider() {
        let mut req = request("00000000-0000-4000-8000-000000000005");
        req.provider_id = "codex".into();
        assert!(validate_request(&req).is_ok());
        req.provider_id = "opencode".into();
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

    #[cfg(unix)]
    #[test]
    fn codex_fake_cli_uses_the_shared_task_run_event_lifecycle() {
        let _fake_cli_guard = fake_cli_test_guard();
        let directory = tempdir().unwrap();
        let workspace = directory.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(
            workspace.join("README.md"),
            "Synthetic acceptance fixture: VOCAMESH synthetic marker.",
        )
        .unwrap();
        let runtime = fake_codex_runtime(directory.path());
        let mut request = runtime_request(
            "00000000-0000-4000-8000-000000000006",
            &workspace,
            "Read the synthetic README",
        );
        request.provider_id = "codex".into();

        let (task, pending) = runtime
            .start_read_only_task(request, |_| {})
            .expect("test-only Codex policy permits the synthetic fixture");
        let finished = wait_for_terminal(&runtime, &pending.id);
        assert_eq!(task.provider_id, "codex");
        assert_eq!(finished.provider_id, "codex");
        assert_eq!(finished.state, RunState::Finished);
        assert_eq!(finished.outcome.as_deref(), Some("success"));
        assert_eq!(
            finished.summary.as_deref(),
            Some("VOCAMESH synthetic marker")
        );
        let events = runtime.list_run_events(&finished.id, 0, 100).unwrap();
        assert!(events
            .iter()
            .any(|event| { event.kind == "final" && event.text == "VOCAMESH synthetic marker" }));
        assert!(!events
            .iter()
            .any(|event| event.text.contains("thread.started")));
    }

    #[cfg(unix)]
    #[test]
    fn codex_protocol_failure_and_missing_completion_cannot_be_success() {
        let _fake_cli_guard = fake_cli_test_guard();
        let directory = tempdir().unwrap();
        let workspace = directory.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let runtime = fake_codex_runtime(directory.path());

        for (request_id, prompt, outcome, reason) in [
            (
                "00000000-0000-4000-8000-000000000008",
                "synthetic failed response",
                "provider-failed",
                "execution.provider-failed",
            ),
            (
                "00000000-0000-4000-8000-000000000009",
                "synthetic missing-completion response",
                "missing-final-result",
                "run.output-protocol-error",
            ),
        ] {
            let mut request = runtime_request(request_id, &workspace, prompt);
            request.provider_id = "codex".into();
            let (_, pending) = runtime.start_read_only_task(request, |_| {}).unwrap();
            let finished = wait_for_terminal(&runtime, &pending.id);
            assert_eq!(finished.state, RunState::Crashed);
            assert_eq!(finished.outcome.as_deref(), Some(outcome));
            assert_eq!(finished.error_code.as_deref(), Some(reason));
        }
    }

    #[cfg(unix)]
    #[test]
    fn codex_fake_cli_uses_shared_stop_and_timeout_supervision() {
        let _fake_cli_guard = fake_cli_test_guard();
        let directory = tempdir().unwrap();
        let workspace = directory.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let runtime = fake_codex_runtime(directory.path());
        let mut request = runtime_request(
            "00000000-0000-4000-8000-000000000010",
            &workspace,
            "slow synthetic fixture",
        );
        request.provider_id = "codex".into();
        let (_, pending) = runtime.start_read_only_task(request, |_| {}).unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while runtime.get_run(&pending.id).unwrap().state != RunState::Running {
            assert!(
                Instant::now() < deadline,
                "fake Codex process did not start"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let stop_requested = runtime.stop_run(&pending.id, |_| {}).unwrap();
        let stopped = wait_for_terminal(&runtime, &pending.id);
        assert_eq!(stop_requested.state, RunState::StopRequested);
        assert_eq!(stopped.outcome.as_deref(), Some("stopped"));

        let timeout_directory = tempdir().unwrap();
        let timeout_workspace = timeout_directory.path().join("workspace");
        std::fs::create_dir_all(&timeout_workspace).unwrap();
        let timeout_runtime =
            fake_codex_runtime_with_timeout(timeout_directory.path(), Duration::from_millis(100));
        let mut timeout_request = runtime_request(
            "00000000-0000-4000-8000-000000000011",
            &timeout_workspace,
            "slow synthetic fixture",
        );
        timeout_request.provider_id = "codex".into();
        let (_, pending) = timeout_runtime
            .start_read_only_task(timeout_request, |_| {})
            .unwrap();
        let timed_out = wait_for_terminal(&timeout_runtime, &pending.id);
        assert_eq!(timed_out.outcome.as_deref(), Some("timeout"));
        assert_eq!(timed_out.error_code.as_deref(), Some("run.timeout"));
    }

    #[test]
    fn production_codex_policy_blocks_before_creating_a_run() {
        let directory = tempdir().unwrap();
        let runtime = ExecutionRuntime::open(
            directory.path().join("config"),
            directory.path().join("data"),
        )
        .unwrap();
        let mut request = request("00000000-0000-4000-8000-000000000007");
        request.provider_id = "codex".into();

        let error = runtime.start_read_only_task(request, |_| {}).unwrap_err();
        assert_eq!(error.code, "execution.codex-safety-unverified");
        assert!(runtime.list_tasks().unwrap().is_empty());
        let readiness = runtime.readiness_for(Provider::Codex);
        assert!(!readiness.ready);
        assert_eq!(
            readiness.reason_code.as_deref(),
            Some("execution.codex-safety-unverified")
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
            Provider::Claude,
        );
        assert_eq!(
            rx.recv().unwrap(),
            ProviderOutput::FinalAndComplete("done".into())
        );
    }

    #[test]
    fn stream_reader_handles_fragmented_eof_non_utf8_and_oversized_lines() {
        let fixture = br#"{"type":"turn.completed"}"#.to_vec();
        let (tx, rx) = mpsc::sync_channel(2);
        read_stream(ChunkedReader::new(fixture, 3), tx, Provider::Codex);
        assert_eq!(rx.recv().unwrap(), ProviderOutput::ProtocolComplete);

        let (tx, rx) = mpsc::sync_channel(2);
        read_stream(std::io::Cursor::new(vec![0xff, b'\n']), tx, Provider::Codex);
        assert_eq!(rx.recv().unwrap(), ProviderOutput::ProtocolError);

        let (tx, rx) = mpsc::sync_channel(2);
        read_stream(
            std::io::Cursor::new(vec![b'x'; STREAM_LINE_LIMIT + 1]),
            tx,
            Provider::Codex,
        );
        assert_eq!(rx.recv().unwrap(), ProviderOutput::ProtocolError);
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
