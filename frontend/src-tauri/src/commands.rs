use std::sync::{atomic::Ordering, Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use vocamesh_agent::AgentRuntime;
use vocamesh_core::{
    ExecutionReadiness, ExecutionRequest, LocalTool, RecoveryInspection, RecoveryResolution,
    RunEvent, RunRecord, ScanPhase, ServiceError, TaskRecord, ToolScanEvent, UnknownRunSummary,
};
use vocamesh_runtime::RuntimeNotification;

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
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut runtime = runtime.lock().map_err(|_| ERR_RUNTIME)?;
        if scan_generation.load(Ordering::SeqCst) != generation {
            return Err(ERR_SCAN_SUPERSEDED);
        }
        runtime.scan_tools()
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
    with_runtime(Arc::clone(&state.runtime), move |runtime| {
        runtime.verify_tool(&id)
    })
    .await
}

#[tauri::command]
pub(crate) async fn set_tool_path(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<LocalTool, ServiceError> {
    with_runtime(Arc::clone(&state.runtime), move |runtime| {
        runtime.set_tool_path(&id, &path)
    })
    .await
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
pub(crate) async fn get_execution_readiness(
    state: State<'_, AppState>,
) -> Result<ExecutionReadiness, ServiceError> {
    let Some(runtime) = state.execution.clone() else {
        return Ok(ExecutionReadiness {
            ready: false,
            version: None,
            auth_state: vocamesh_core::AuthenticationState::Unknown,
            message: ERR_STORAGE.message.into(),
            reason_code: Some(ERR_STORAGE.code.into()),
        });
    };
    tauri::async_runtime::spawn_blocking(move || Ok(runtime.readiness()))
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
