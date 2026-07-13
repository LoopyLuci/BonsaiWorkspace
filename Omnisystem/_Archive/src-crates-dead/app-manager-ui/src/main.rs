// Tauri desktop application entry point

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod api;
mod models;
mod state;

use tauri::Manager;
use tracing_subscriber;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .setup(|app| {
            let _app_handle = app.app_handle();

            tracing::info!("App Manager UI starting up");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api::auth::login,
            api::auth::logout,
            api::auth::verify_token,
            api::apps::list_apps,
            api::apps::search_apps,
            api::apps::get_app,
            api::apps::install_app,
            api::apps::uninstall_app,
            api::marketplace::rate_app,
            api::marketplace::get_reviews,
            api::marketplace::get_trending,
            api::marketplace::get_featured,
            api::settings::get_settings,
            api::settings::update_settings,
            api::health::check_api_health,
            api::statistics::get_installation_stats,
            api::statistics::get_usage_statistics,
            api::favorites::add_favorite,
            api::favorites::remove_favorite,
            api::favorites::get_favorites,
            api::favorites::is_favorite,
            api::telemetry::track_event,
            api::telemetry::get_telemetry_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
