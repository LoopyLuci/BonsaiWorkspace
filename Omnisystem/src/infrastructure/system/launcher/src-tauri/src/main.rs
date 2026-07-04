#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;
mod service_monitor;
mod app_registry;

use tauri::{
    AppHandle, Manager, WindowEvent,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String,
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize app registry
            let app_registry = Arc::new(app_registry::AppRegistryClient::new(
                "http://localhost:11369".to_string(),
            ));
            app.manage(app_registry);

            // Initialize service monitor
            let service_monitor = Arc::new(Mutex::new(
                service_monitor::ServiceMonitor::new("http://localhost:11369".to_string()),
            ));
            app.manage(service_monitor);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Prevent default close on some windows (minimize to tray instead)
                if window.label() == "main"
                    || window.label() == "quick-panel"
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            launch_app,
            get_apps,
            search_apps,
            get_service_status,
            open_control_panel,
            get_featured_apps,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn launch_app(
    app_id: String,
    app_registry: tauri::State<'_, Arc<app_registry::AppRegistryClient>>,
) -> Result<(), String> {
    app_registry.launch_app(&app_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_apps(
    app_registry: tauri::State<'_, Arc<app_registry::AppRegistryClient>>,
) -> Result<Vec<serde_json::Value>, String> {
    app_registry.get_all_apps().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_apps(
    query: String,
    app_registry: tauri::State<'_, Arc<app_registry::AppRegistryClient>>,
) -> Result<Vec<serde_json::Value>, String> {
    app_registry.search_apps(&query).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_featured_apps(
    app_registry: tauri::State<'_, Arc<app_registry::AppRegistryClient>>,
) -> Result<Vec<serde_json::Value>, String> {
    app_registry.get_featured_apps().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_service_status(
    service_monitor: tauri::State<'_, Arc<Mutex<service_monitor::ServiceMonitor>>>,
) -> Result<serde_json::Value, String> {
    let monitor = service_monitor.lock().await;
    monitor.get_status().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn open_control_panel(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("control-panel") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}
