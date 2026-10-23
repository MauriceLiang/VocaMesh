mod commands;

use std::sync::{atomic::AtomicU64, Arc, Mutex};
use tauri::Manager;
use vocamesh_agent::AgentRuntime;
use vocamesh_runtime::ExecutionRuntime;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) runtime: Arc<Mutex<AgentRuntime>>,
    pub(crate) scan_generation: Arc<AtomicU64>,
    pub(crate) execution: Option<ExecutionRuntime>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let execution = ExecutionRuntime::open(config_dir.clone(), data_dir).ok();
            app.manage(AppState {
                runtime: Arc::new(Mutex::new(AgentRuntime::new(config_dir))),
                scan_generation: Arc::new(AtomicU64::new(0)),
                execution,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_tools,
            commands::verify_tool,
            commands::set_tool_path,
            commands::get_default_tool_id,
            commands::set_default_tool,
            commands::select_workspace_directory,
            commands::get_catalog_status,
            commands::list_agents,
            commands::get_agent,
            commands::create_agent,
            commands::update_agent,
            commands::set_agent_permissions,
            commands::archive_agent,
            commands::list_workspaces,
            commands::get_workspace,
            commands::create_workspace,
            commands::update_workspace_scopes,
            commands::rename_workspace,
            commands::archive_workspace,
            commands::list_teams,
            commands::get_team,
            commands::list_team_members,
            commands::create_team,
            commands::update_team,
            commands::add_team_member,
            commands::remove_team_member,
            commands::set_team_reports_to,
            commands::get_appearance_settings,
            commands::update_appearance_settings,
            commands::get_execution_readiness,
            commands::list_unknown_runs,
            commands::inspect_unknown_run,
            commands::resolve_unknown_run,
            commands::start_read_only_task,
            commands::stop_run,
            commands::get_run,
            commands::list_run_events,
            commands::list_tasks,
            commands::get_task,
            commands::list_runs,
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                if let Some(execution) = &window.state::<AppState>().execution {
                    execution.shutdown();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run VocaMesh desktop application");
}
