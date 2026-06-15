// Tauri commands - IPC interface to launcher daemon

use crate::models::*;
use crate::state::LauncherState;
use serde_json::{json, Value};
use tauri::State;

// ============================================================================
// App Management Commands
// ============================================================================

#[tauri::command]
pub async fn list_apps(state: State<'_, LauncherState>) -> Result<Vec<AppInfo>, String> {
    log::debug!("Listing all applications");
    state.list_apps().await
}

#[tauri::command]
pub async fn search_apps(query: String, state: State<'_, LauncherState>) -> Result<Vec<AppInfo>, String> {
    log::debug!("Searching apps with query: {}", query);
    state.search_apps(&query).await
}

#[tauri::command]
pub async fn launch_app(app_id: String, state: State<'_, LauncherState>) -> Result<LaunchResult, String> {
    log::info!("Launching app: {}", app_id);
    state.launch_app(&app_id).await
}

#[tauri::command]
pub async fn get_app_details(app_id: String, state: State<'_, LauncherState>) -> Result<AppInfo, String> {
    log::debug!("Getting details for app: {}", app_id);
    state.get_app_details(&app_id).await
}

#[tauri::command]
pub async fn terminate_app(instance_id: String, state: State<'_, LauncherState>) -> Result<(), String> {
    log::info!("Terminating instance: {}", instance_id);
    state.terminate_app(&instance_id).await
}

#[tauri::command]
pub async fn get_running_instances(state: State<'_, LauncherState>) -> Result<Vec<AppInstance>, String> {
    log::debug!("Getting running instances");
    state.get_running_instances().await
}

// ============================================================================
// System Commands
// ============================================================================

#[tauri::command]
pub async fn get_system_status(state: State<'_, LauncherState>) -> Result<SystemStatus, String> {
    log::debug!("Getting system status");
    state.get_system_status().await
}

#[tauri::command]
pub async fn get_daemon_status(state: State<'_, LauncherState>) -> Result<DaemonStatus, String> {
    log::debug!("Getting daemon status");
    state.get_daemon_status().await
}

#[tauri::command]
pub async fn get_launcher_config(state: State<'_, LauncherState>) -> Result<LauncherConfig, String> {
    log::debug!("Getting launcher configuration");
    state.get_launcher_config().await
}

#[tauri::command]
pub async fn update_launcher_config(
    config: LauncherConfig,
    state: State<'_, LauncherState>,
) -> Result<(), String> {
    log::info!("Updating launcher configuration");
    state.update_launcher_config(config).await
}

// ============================================================================
// UI Commands
// ============================================================================

#[tauri::command]
pub async fn open_quick_panel(window: tauri::Window) -> Result<(), String> {
    log::debug!("Opening quick panel");
    if let Ok(quick_panel) = window.get_window("quick-panel") {
        let _ = quick_panel.show();
        let _ = quick_panel.set_focus();
    }
    Ok(())
}

#[tauri::command]
pub async fn close_quick_panel(window: tauri::Window) -> Result<(), String> {
    log::debug!("Closing quick panel");
    if let Ok(quick_panel) = window.get_window("quick-panel") {
        let _ = quick_panel.hide();
    }
    Ok(())
}

#[tauri::command]
pub async fn toggle_always_on_top(window: tauri::Window) -> Result<(), String> {
    log::debug!("Toggling always on top");
    let main = window.get_window("main").map_err(|e| e.to_string())?;
    let current = main.is_always_on_top().map_err(|e| e.to_string())?;
    main.set_always_on_top(!current).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_window_state(window: tauri::Window) -> Result<WindowState, String> {
    log::debug!("Getting window state");
    let main = window.get_window("main").map_err(|e| e.to_string())?;

    let (width, height) = main
        .inner_size()
        .map(|size| (size.width, size.height))
        .unwrap_or((1200, 800));

    let (x, y) = main
        .outer_position()
        .map(|pos| (pos.x, pos.y))
        .unwrap_or((100, 100));

    let is_maximized = main.is_maximized().unwrap_or(false);
    let is_always_on_top = main.is_always_on_top().unwrap_or(false);

    Ok(WindowState {
        width,
        height,
        x,
        y,
        maximized: is_maximized,
        always_on_top: is_always_on_top,
    })
}

// ============================================================================
// Logging Commands
// ============================================================================

#[tauri::command]
pub async fn log_event(level: String, message: String) -> Result<(), String> {
    match level.to_lowercase().as_str() {
        "debug" => log::debug!("{}", message),
        "info" => log::info!("{}", message),
        "warn" => log::warn!("{}", message),
        "error" => log::error!("{}", message),
        _ => log::info!("{}", message),
    }
    Ok(())
}

#[tauri::command]
pub async fn get_logs(state: State<'_, LauncherState>, count: Option<usize>) -> Result<Vec<LogEntry>, String> {
    let limit = count.unwrap_or(100);
    state.get_logs(limit).await
}
