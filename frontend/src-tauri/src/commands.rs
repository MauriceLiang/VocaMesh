use std::sync::{atomic::Ordering, Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use vocamesh_agent::AgentRuntime;
use vocamesh_core::{
    derive_tool_state, AddTeamMemberInput, AgentPermissions, AgentRole, AppearanceSettings,
    CatalogStatus, CreateAgentInput, CreateTeamInput, CreateWorkspaceInput, ExecutionReadiness,
    ExecutionRequest, LocalTool, Provider, RecoveryInspection, RecoveryResolution, RunEvent,
    RunRecord, ScanPhase, ServiceError, TaskRecord, TeamMemberRecord, TeamRecord,
    ToolIntegrationStatus, ToolScanEvent, UnknownRunSummary, UpdateAgentInput, UpdateTeamInput,
    UpdateWorkspaceScopesInput, WorkspaceRecord,
};
use vocamesh_runtime::{ExecutionRuntime, RuntimeNotification};

use crate::AppState;

const ERR_RUNTIME: ServiceError = ServiceError::new(
    "execution.runtime-failed",
    "本机任务服务暂时不可用，请重启应用",
);
const ERR_SCAN_SUPERSEDED: ServiceError =
    ServiceError::new("tool.scan-superseded", "扫描已被更新请求替代");
const ERR_STORAGE: ServiceError = ServiceError::new(
    "storage.read-failed",
    "本地任务数据库无法读取；原文件已保留，请检查数据库后重启应用",
);

async fn with_runtime<T, F>(
    runtime: Arc<Mutex<AgentRuntime>>,
    operation: F,
) -> Result<T, ServiceError>
where
    T: Send + 'static,
    F: FnOnce(&mut AgentRuntime) -> Result<T, ServiceError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let mut runtime = runtime.lock().map_err(|_| ERR_RUNTIME)?;
        operation(&mut runtime)
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

async fn with_catalog<T, F>(state: State<'_, AppState>, operation: F) -> Result<T, ServiceError>
where
    T: Send + 'static,
    F: FnOnce(ExecutionRuntime) -> Result<T, ServiceError> + Send + 'static,
{
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || operation(runtime))
        .await
        .map_err(|_| ERR_RUNTIME)?
}

fn apply_runtime_readiness(
    mut tool: LocalTool,
    readiness: Option<&ExecutionReadiness>,
) -> LocalTool {
    if tool.capabilities.execution_adapter_available {
        if let Some(readiness) = readiness {
            tool.runtime_ready = readiness.ready;
            tool.runtime_reason_code = readiness.reason_code.clone();
            if readiness.ready {
                tool.auth_state = vocamesh_core::AuthenticationState::Authenticated;
            } else if readiness.reason_code.as_deref() == Some("execution.auth-required") {
                tool.auth_state = vocamesh_core::AuthenticationState::Unauthenticated;
            } else if readiness.reason_code.as_deref() == Some("execution.auth-unknown") {
                tool.auth_state = vocamesh_core::AuthenticationState::Unknown;
            }
            match readiness.reason_code.as_deref() {
                Some("execution.managed-policy-unverified") => {
                    tool.integration_status = ToolIntegrationStatus::BlockedByPolicy;
                }
                Some("execution.codex-safety-unverified") => {
                    tool.integration_status = ToolIntegrationStatus::BlockedByPolicy;
                }
                Some("execution.platform-unsupported") => {
                    tool.integration_status = ToolIntegrationStatus::UnsupportedPlatform;
                }
                _ => {}
            }
        } else {
            tool.runtime_ready = false;
            tool.runtime_reason_code = Some("storage.read-failed".to_owned());
        }
    } else {
        tool.runtime_ready = false;
        tool.runtime_reason_code = tool
            .capabilities
            .unsupported_reason_code
            .clone()
            .or_else(|| Some("tool.adapter-unavailable".to_owned()));
    }
    tool.state = derive_tool_state(&tool);
    tool
}

fn runtime_readiness_for(
    execution: Option<&ExecutionRuntime>,
    provider_id: &str,
) -> Option<ExecutionReadiness> {
    let runtime = execution?;
    let provider = provider_id.parse::<Provider>().ok()?;
    Some(runtime.readiness_for(provider))
}

#[tauri::command]
pub(crate) async fn scan_tools(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<LocalTool>, ServiceError> {
    let generation = state.scan_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = app.emit(
        "tool-scan",
        ToolScanEvent {
            phase: ScanPhase::Scanning,
            message: Some("正在检查 Codex、Claude Code 和 OpenCode"),
        },
    );
    let runtime = Arc::clone(&state.runtime);
    let scan_generation = Arc::clone(&state.scan_generation);
    let execution = state.execution.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut runtime = runtime.lock().map_err(|_| ERR_RUNTIME)?;
        if scan_generation.load(Ordering::SeqCst) != generation {
            return Err(ERR_SCAN_SUPERSEDED);
        }
        runtime.scan_tools().map(|tools| {
            tools
                .into_iter()
                .map(|tool| {
                    let readiness = runtime_readiness_for(execution.as_ref(), &tool.id);
                    apply_runtime_readiness(tool, readiness.as_ref())
                })
                .collect()
        })
    })
    .await
    .map_err(|_| ERR_RUNTIME)?;
    if state.scan_generation.load(Ordering::SeqCst) == generation {
        match &result {
            Ok(_) => {
                let _ = app.emit(
                    "tool-scan",
                    ToolScanEvent {
                        phase: ScanPhase::Done,
                        message: Some("本机工具扫描完成"),
                    },
                );
            }
            Err(error) => {
                let _ = app.emit(
                    "tool-scan",
                    ToolScanEvent {
                        phase: ScanPhase::Failed,
                        message: Some(error.message),
                    },
                );
            }
        }
    }
    result
}

#[tauri::command]
pub(crate) async fn verify_tool(
    state: State<'_, AppState>,
    id: String,
) -> Result<LocalTool, ServiceError> {
    let runtime = Arc::clone(&state.runtime);
    let execution = state.execution.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut runtime = runtime.lock().map_err(|_| ERR_RUNTIME)?;
        runtime.verify_tool(&id).map(|tool| {
            let readiness = runtime_readiness_for(execution.as_ref(), &tool.id);
            apply_runtime_readiness(tool, readiness.as_ref())
        })
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn set_tool_path(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<LocalTool, ServiceError> {
    let runtime = Arc::clone(&state.runtime);
    let execution = state.execution.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut runtime = runtime.lock().map_err(|_| ERR_RUNTIME)?;
        runtime.set_tool_path(&id, &path).map(|tool| {
            let readiness = runtime_readiness_for(execution.as_ref(), &tool.id);
            apply_runtime_readiness(tool, readiness.as_ref())
        })
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn get_default_tool_id(
    state: State<'_, AppState>,
) -> Result<Option<String>, ServiceError> {
    let runtime = Arc::clone(&state.runtime);
    tauri::async_runtime::spawn_blocking(move || {
        runtime.lock().map_err(|_| ERR_RUNTIME)?.default_tool_id()
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn set_default_tool(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), ServiceError> {
    with_runtime(Arc::clone(&state.runtime), move |runtime| {
        runtime.set_default_tool(&id)
    })
    .await
}

#[tauri::command]
pub(crate) async fn select_workspace_directory(
    app: AppHandle,
) -> Result<Option<String>, ServiceError> {
    let path = tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        app.dialog()
            .file()
            .set_title("选择只读工作目录")
            .blocking_pick_folder()
            .and_then(|selected| selected.into_path().ok())
    })
    .await
    .map_err(|_| ERR_RUNTIME)?;
    Ok(path.map(|path| path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub(crate) async fn get_catalog_status(
    state: State<'_, AppState>,
) -> Result<CatalogStatus, ServiceError> {
    with_catalog(state, |runtime| runtime.catalog_status()).await
}

#[tauri::command]
pub(crate) async fn list_agents(
    state: State<'_, AppState>,
) -> Result<Vec<AgentRole>, ServiceError> {
    with_catalog(state, |runtime| runtime.list_agents()).await
}

#[tauri::command]
pub(crate) async fn get_agent(
    state: State<'_, AppState>,
    id: String,
) -> Result<AgentRole, ServiceError> {
    with_catalog(state, move |runtime| runtime.get_agent(&id)).await
}

#[tauri::command]
pub(crate) async fn create_agent(
    state: State<'_, AppState>,
    input: CreateAgentInput,
) -> Result<AgentRole, ServiceError> {
    with_catalog(state, move |runtime| runtime.create_agent(input)).await
}

#[tauri::command]
pub(crate) async fn update_agent(
    state: State<'_, AppState>,
    id: String,
    input: UpdateAgentInput,
) -> Result<AgentRole, ServiceError> {
    with_catalog(state, move |runtime| runtime.update_agent(&id, input)).await
}

#[tauri::command]
pub(crate) async fn set_agent_permissions(
    state: State<'_, AppState>,
    id: String,
    permissions: AgentPermissions,
    expected_revision: Option<u64>,
) -> Result<AgentRole, ServiceError> {
    with_catalog(state, move |runtime| {
        runtime.update_agent(
            &id,
            UpdateAgentInput {
                permissions: Some(permissions),
                expected_revision,
                ..Default::default()
            },
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn archive_agent(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), ServiceError> {
    with_catalog(state, move |runtime| runtime.archive_agent(&id)).await
}

#[tauri::command]
pub(crate) async fn list_workspaces(
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceRecord>, ServiceError> {
    with_catalog(state, |runtime| runtime.list_workspaces()).await
}

#[tauri::command]
pub(crate) async fn get_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<WorkspaceRecord, ServiceError> {
    with_catalog(state, move |runtime| runtime.get_workspace(&id)).await
}

#[tauri::command]
pub(crate) async fn create_workspace(
    state: State<'_, AppState>,
    input: CreateWorkspaceInput,
) -> Result<WorkspaceRecord, ServiceError> {
    with_catalog(state, move |runtime| runtime.create_workspace(input)).await
}

#[tauri::command]
pub(crate) async fn update_workspace_scopes(
    state: State<'_, AppState>,
    id: String,
    input: UpdateWorkspaceScopesInput,
) -> Result<WorkspaceRecord, ServiceError> {
    with_catalog(state, move |runtime| {
        runtime.update_workspace_scopes(&id, input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn rename_workspace(
    state: State<'_, AppState>,
    id: String,
    name: String,
    expected_revision: Option<u64>,
) -> Result<WorkspaceRecord, ServiceError> {
    with_catalog(state, move |runtime| {
        runtime.rename_workspace(&id, &name, expected_revision)
    })
    .await
}

#[tauri::command]
pub(crate) async fn archive_workspace(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), ServiceError> {
    with_catalog(state, move |runtime| runtime.archive_workspace(&id)).await
}

#[tauri::command]
pub(crate) async fn list_teams(
    state: State<'_, AppState>,
) -> Result<Vec<TeamRecord>, ServiceError> {
    with_catalog(state, |runtime| runtime.list_teams()).await
}

#[tauri::command]
pub(crate) async fn get_team(
    state: State<'_, AppState>,
    id: String,
) -> Result<TeamRecord, ServiceError> {
    with_catalog(state, move |runtime| runtime.get_team(&id)).await
}

#[tauri::command]
pub(crate) async fn list_team_members(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<TeamMemberRecord>, ServiceError> {
    with_catalog(state, move |runtime| {
        runtime.get_team(&id).map(|team| team.members)
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_team(
    state: State<'_, AppState>,
    input: CreateTeamInput,
) -> Result<TeamRecord, ServiceError> {
    with_catalog(state, move |runtime| runtime.create_team(input)).await
}

#[tauri::command]
pub(crate) async fn update_team(
    state: State<'_, AppState>,
    id: String,
    input: UpdateTeamInput,
) -> Result<TeamRecord, ServiceError> {
    with_catalog(state, move |runtime| runtime.update_team(&id, input)).await
}

#[tauri::command]
pub(crate) async fn add_team_member(
    state: State<'_, AppState>,
    id: String,
    input: AddTeamMemberInput,
) -> Result<TeamRecord, ServiceError> {
    with_catalog(state, move |runtime| runtime.add_team_member(&id, input)).await
}

#[tauri::command]
pub(crate) async fn remove_team_member(
    state: State<'_, AppState>,
    id: String,
    agent_id: String,
) -> Result<TeamRecord, ServiceError> {
    with_catalog(state, move |runtime| {
        runtime.remove_team_member(&id, &agent_id)
    })
    .await
}

#[tauri::command]
pub(crate) async fn set_team_reports_to(
    state: State<'_, AppState>,
    id: String,
    agent_id: String,
    reports_to_agent_id: String,
) -> Result<TeamRecord, ServiceError> {
    with_catalog(state, move |runtime| {
        runtime.set_team_reports_to(&id, &agent_id, &reports_to_agent_id)
    })
    .await
}

#[tauri::command]
pub(crate) async fn get_appearance_settings(
    state: State<'_, AppState>,
) -> Result<AppearanceSettings, ServiceError> {
    with_catalog(state, |runtime| runtime.appearance_settings()).await
}

#[tauri::command]
pub(crate) async fn update_appearance_settings(
    state: State<'_, AppState>,
    settings: AppearanceSettings,
) -> Result<AppearanceSettings, ServiceError> {
    with_catalog(state, move |runtime| {
        runtime.update_appearance_settings(settings)
    })
    .await
}

#[tauri::command]
pub(crate) async fn get_execution_readiness(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<ExecutionReadiness, ServiceError> {
    let provider = provider_id.parse::<Provider>().map_err(|_| {
        ServiceError::new("execution.provider-not-supported", "Provider 当前不可执行")
    })?;
    let Some(runtime) = state.execution.clone() else {
        return Ok(ExecutionReadiness {
            ready: false,
            version: None,
            auth_state: if provider == Provider::Codex {
                vocamesh_core::AuthenticationState::Unsupported
            } else {
                vocamesh_core::AuthenticationState::Unknown
            },
            message: ERR_STORAGE.message.into(),
            reason_code: Some(ERR_STORAGE.code.into()),
        });
    };
    tauri::async_runtime::spawn_blocking(move || Ok(runtime.readiness_for(provider)))
        .await
        .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn list_unknown_runs(
    state: State<'_, AppState>,
) -> Result<Vec<UnknownRunSummary>, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || runtime.list_unknown_runs())
        .await
        .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn inspect_unknown_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<RecoveryInspection, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || runtime.inspect_unknown_run(&run_id))
        .await
        .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn resolve_unknown_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
    user_confirmed: bool,
) -> Result<RecoveryResolution, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || {
        let resolution = runtime.resolve_unknown_run(&run_id, user_confirmed)?;
        let _ = app.emit("run://state", resolution.run.clone());
        Ok(resolution)
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn start_read_only_task(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ExecutionRequest,
) -> Result<StartTaskResponse, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || {
        let notify_app = app.clone();
        let (task, run) = runtime.start_read_only_task(request, move |notification| {
            emit_runtime(&notify_app, notification)
        })?;
        Ok(StartTaskResponse { task, run })
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn stop_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<RunRecord, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || {
        let notify_app = app.clone();
        runtime.stop_run(&run_id, move |notification| {
            emit_runtime(&notify_app, notification)
        })
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn get_run(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<RunRecord, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || runtime.get_run(&run_id))
        .await
        .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn list_run_events(
    state: State<'_, AppState>,
    run_id: String,
    after: Option<u64>,
    limit: Option<usize>,
) -> Result<Vec<RunEvent>, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || {
        runtime.list_run_events(&run_id, after.unwrap_or(0), limit.unwrap_or(100))
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn list_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<TaskRecord>, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || runtime.list_tasks())
        .await
        .map_err(|_| ERR_RUNTIME)?
}

#[tauri::command]
pub(crate) async fn get_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<TaskRecord, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || runtime.get_task(&task_id))
        .await
        .map_err(|_| ERR_RUNTIME)?
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartTaskResponse {
    task: TaskRecord,
    run: RunRecord,
}

#[tauri::command]
pub(crate) async fn list_runs(
    state: State<'_, AppState>,
    cursor: Option<u64>,
    limit: Option<usize>,
) -> Result<RunPage, ServiceError> {
    let runtime = state.execution.clone().ok_or(ERR_STORAGE)?;
    tauri::async_runtime::spawn_blocking(move || {
        let limit = limit.unwrap_or(50).clamp(1, 100);
        let page = runtime.list_runs(cursor, limit)?;
        Ok(RunPage {
            items: page.items,
            has_more: page.has_more,
            next_cursor: page.next_cursor,
        })
    })
    .await
    .map_err(|_| ERR_RUNTIME)?
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunPage {
    items: Vec<RunRecord>,
    has_more: bool,
    next_cursor: Option<u64>,
}

fn emit_runtime(app: &AppHandle, notification: RuntimeNotification) {
    match notification {
        RuntimeNotification::State(run) => {
            let _ = app.emit("run://state", run);
        }
        RuntimeNotification::Event(event) => {
            let _ = app.emit("run://event", event);
        }
    }
}

#[cfg(test)]
mod tool_readiness_tests {
    use super::*;
    use vocamesh_core::{
        AuthenticationState, Provider, ToolDiscoveryStatus, ToolState, ToolVersionStatus,
    };

    fn discovered_claude() -> LocalTool {
        let mut tool = LocalTool::unavailable(Provider::Claude, ToolState::Undetectable, "test");
        tool.discovery_status = ToolDiscoveryStatus::Found;
        tool.version_status = ToolVersionStatus::Known;
        tool.integration_status = ToolIntegrationStatus::AdapterAvailable;
        tool.auth_state = AuthenticationState::Authenticated;
        tool
    }

    #[test]
    fn managed_policy_readiness_is_overlaid_without_marking_the_tool_ready() {
        let readiness = ExecutionReadiness {
            ready: false,
            version: Some("2.1.296".into()),
            auth_state: AuthenticationState::Unknown,
            message: "policy not verified".into(),
            reason_code: Some("execution.managed-policy-unverified".into()),
        };

        let tool = apply_runtime_readiness(discovered_claude(), Some(&readiness));

        assert_eq!(
            tool.integration_status,
            ToolIntegrationStatus::BlockedByPolicy
        );
        assert_eq!(
            tool.runtime_reason_code.as_deref(),
            Some("execution.managed-policy-unverified")
        );
        assert!(!tool.runtime_ready);
        assert_eq!(tool.state, ToolState::Undetectable);
        assert_eq!(tool.auth_state, AuthenticationState::Authenticated);
    }

    #[test]
    fn codex_adapter_is_shown_as_blocked_until_its_own_policy_is_proven() {
        let readiness = ExecutionReadiness {
            ready: false,
            version: Some("0.162.1".into()),
            auth_state: AuthenticationState::Unsupported,
            message: "Codex policy unknown".into(),
            reason_code: Some("execution.codex-safety-unverified".into()),
        };
        let mut codex = LocalTool::unavailable(Provider::Codex, ToolState::Missing, "test");
        codex.discovery_status = ToolDiscoveryStatus::Found;
        codex.version_status = ToolVersionStatus::Known;

        let codex = apply_runtime_readiness(codex, Some(&readiness));

        assert!(!codex.runtime_ready);
        assert_eq!(
            codex.integration_status,
            ToolIntegrationStatus::BlockedByPolicy
        );
        assert_eq!(codex.auth_state, AuthenticationState::Unsupported);
        assert_eq!(
            codex.runtime_reason_code.as_deref(),
            Some("execution.codex-safety-unverified")
        );
        assert_eq!(codex.state, ToolState::Undetectable);
    }
}
