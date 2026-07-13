use axum::{
    routing::{get, delete},
    Router,
};
use crate::handlers::{
    get_apps, get_app, install_app, uninstall_app, start_app, stop_app, update_app, configure_app,
    get_app_logs, get_system_health, search_marketplace,
};

pub fn create_router() -> Router {
    Router::new()
        .route("/api/v1/apps", get(get_apps))
        .route("/api/v1/apps/:app_id", get(get_app))
        .route("/api/v1/apps/:app_id/install", get(install_app))
        .route("/api/v1/apps/:app_id", delete(uninstall_app))
        .route("/api/v1/apps/:app_id/start", get(start_app))
        .route("/api/v1/apps/:app_id/stop", get(stop_app))
        .route("/api/v1/apps/:app_id/update", get(update_app))
        .route("/api/v1/apps/:app_id/config", get(configure_app))
        .route("/api/v1/apps/:app_id/logs", get(get_app_logs))
        .route("/api/v1/system/health", get(get_system_health))
        .route("/api/v1/marketplace/search", get(search_marketplace))
}
