mod capability;
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::check_installation,
            commands::fetch_manifest,
            commands::plan_install,
            commands::verify_manifest,
            commands::execute_install,
            commands::rollback_latest,
            commands::check_for_updates,
            commands::update_components,
            commands::repair_installation,
            commands::get_install_history,
            commands::universe_rollback,
            commands::get_health_report,
            commands::get_settings,
            commands::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running bonsai-root");
}
