use std::sync::{atomic::Ordering, Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use vocamesh_agent::AgentRuntime;
use vocamesh_core::{LocalTool, ScanPhase, ServiceError, ToolScanEvent};

use crate::AppState;

const ERR_RUNTIME: ServiceError =
    ServiceError::new("tool.scan-failed", "本机工具服务暂时不可用，请重新扫描");
const ERR_SCAN_SUPERSEDED: ServiceError =
    ServiceError::new("tool.scan-superseded", "扫描已被更新请求替代");

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
