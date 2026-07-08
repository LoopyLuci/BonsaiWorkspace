//! API route definitions
//! Wires up all endpoints for Environments and Modules APIs

use crate::handlers::{
    create_environment, delete_environment, execute_command, get_environment_status,
    list_environments, migrate_environment, restore_environment, snapshot_environment,
    start_environment, stop_environment, get_module_info, get_operation_progress, install_module,
    search_modules, uninstall_module, update_module, verify_module_signature, init_store,
    init_registry,
    // Service management handlers (Phase 1)
    list_services, start_service, stop_service, get_service_status,
    restart_service, configure_service, snapshot_service, get_service_logs,
    init_service_store,
};
use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Create the main API router with all endpoints
pub fn create_router() -> Router {
    // Initialize shared state
    let env_store = init_store();
    let module_registry = init_registry();
    let service_store = init_service_store();

    // Environment routes
    let environment_routes = Router::new()
        // GET /environments - List environments
        .route("/", get(list_environments))
        // POST /environments - Create environment
        .route("/", post(create_environment))
        // POST /environments/{id}/start - Start environment
        .route("/:id/start", post(start_environment))
        // POST /environments/{id}/stop - Stop environment
        .route("/:id/stop", post(stop_environment))
        // POST /environments/{id}/snapshot - Create snapshot
        .route("/:id/snapshot", post(snapshot_environment))
        // POST /environments/{id}/restore - Restore from snapshot
        .route("/:id/restore", post(restore_environment))
        // POST /environments/{id}/migrate - Migrate environment
        .route("/:id/migrate", post(migrate_environment))
        // DELETE /environments/{id} - Delete environment
        .route("/:id", delete(delete_environment))
        // GET /environments/{id}/status - Get status
        .route("/:id/status", get(get_environment_status))
        // POST /environments/{id}/exec - Execute command
        .route("/:id/exec", post(execute_command))
        .with_state(env_store);

    // Module routes
    let module_routes = Router::new()
        // GET /modules - Search modules
        .route("/", get(search_modules))
        // POST /modules/install - Install module
        .route("/install", post(install_module))
        // POST /modules/{name}/update - Update module
        .route("/:name/update", post(update_module))
        // DELETE /modules/{name} - Uninstall module
        .route("/:name", delete(uninstall_module))
        // GET /modules/{name}/{version} - Get module info
        .route("/:name/:version", get(get_module_info))
        // POST /modules/verify - Verify signature
        .route("/verify", post(verify_module_signature))
        .with_state(module_registry);

    // Service management routes (Phase 1 - 8 endpoints)
    let service_routes = Router::new()
        // GET /services - List all services
        .route("/", get(list_services))
        // POST /services/{name}/start - Start a service
        .route("/:name/start", post(start_service))
        // POST /services/{name}/stop - Stop a service
        .route("/:name/stop", post(stop_service))
        // GET /services/{name}/status - Get service status
        .route("/:name/status", get(get_service_status))
        // POST /services/{name}/restart - Restart a service
        .route("/:name/restart", post(restart_service))
        // POST /services/{name}/configure - Configure a service
        .route("/:name/configure", post(configure_service))
        // POST /services/{name}/snapshot - Create service snapshot
        .route("/:name/snapshot", post(snapshot_service))
        // GET /services/{name}/logs - Get service logs
        .route("/:name/logs", get(get_service_logs))
        .with_state(service_store);

    // Operations routes for async progress tracking
    let operations_routes = Router::new()
        // GET /operations/{task_id}/progress - Get operation progress
        .route("/:task_id/progress", get(get_operation_progress));

    // Combine all routes
    Router::new()
        .nest("/services", service_routes)
        .nest("/environments", environment_routes)
        .nest("/modules", module_routes)
        .nest("/operations", operations_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let _router = create_router();
        // Router should be created without errors
    }
}
