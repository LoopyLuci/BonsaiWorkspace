// Omnisystem Launcher - Native Desktop Application
// Built with Tauri + Svelte for high performance and stability

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde_json::json;
use tauri::Manager;

mod commands;
mod ipc;
mod state;
mod models;

use commands::*;
use state::LauncherState;

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .setup(|app| {
            // Initialize application state
            let state = LauncherState::new();
            app.manage(state);

            // Log startup
            log::info!("Omnisystem Launcher starting up");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // App management commands
            list_apps,
            search_apps,
            launch_app,
            get_app_details,
            terminate_app,
            get_running_instances,

            // System commands
            get_system_status,
            get_daemon_status,
            get_launcher_config,
            update_launcher_config,

            // UI commands
            open_quick_panel,
            close_quick_panel,
            toggle_always_on_top,
            get_window_state,

            // Logging
            log_event,
            get_logs,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Omnisystem Launcher");
}
