mod commands;

use std::sync::{atomic::AtomicU64, Arc, Mutex};
use tauri::Manager;
use vocamesh_agent::AgentRuntime;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) runtime: Arc<Mutex<AgentRuntime>>,
    pub(crate) scan_generation: Arc<AtomicU64>,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            app.manage(AppState {
                runtime: Arc::new(Mutex::new(AgentRuntime::new(config_dir))),
                scan_generation: Arc::new(AtomicU64::new(0)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_tools,
            commands::verify_tool,
            commands::set_tool_path,
            commands::get_default_tool_id,
            commands::set_default_tool,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run VocaMesh desktop application");
}
