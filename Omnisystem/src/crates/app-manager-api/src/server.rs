//! HTTP API Server implementation using Axum

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use app_manager_core::{
    AppRegistry, ModuleRegistry, AppDiscoveryService,
    SearchEngine, DiscoveryFilter, AppId,
};

/// API Server state
#[derive(Clone)]
pub struct ApiState {
    pub app_registry: Arc<AppRegistry>,
    pub module_registry: Arc<ModuleRegistry>,
    pub discovery_service: Arc<AppDiscoveryService>,
}

/// Generic API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// App info for API responses
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppInfoResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub rating: f32,
    pub downloads: u32,
    pub installed: bool,
}

/// Discovery query parameters
#[derive(Debug, Deserialize)]
pub struct DiscoveryQuery {
    pub name: Option<String>,
    pub category: Option<String>,
    pub min_rating: Option<f32>,
    pub limit: Option<usize>,
}

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

// ============================================================================
// Handler Functions
// ============================================================================

/// GET /api/health - Health check
pub async fn health_check() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// GET /api/apps - List all apps
pub async fn list_apps(State(state): State<ApiState>) -> Json<ApiResponse<Vec<AppInfoResponse>>> {
    let apps = state.discovery_service.discover_all();
    let response: Vec<AppInfoResponse> = apps
        .iter()
        .map(|app| AppInfoResponse {
            id: app.manifest.id.to_string(),
            name: app.manifest.name.clone(),
            version: app.manifest.version.to_string(),
            description: app.manifest.description.clone(),
            rating: app.rating,
            downloads: app.download_count,
            installed: app.installed,
        })
        .collect();

    Json(ApiResponse::ok(response))
}

/// GET /api/apps/:id - Get app details
pub async fn get_app(
    State(state): State<ApiState>,
    Path(app_id_str): Path<String>,
) -> impl IntoResponse {
    match app_id_str.parse::<uuid::Uuid>() {
        Ok(uuid) => {
            let app_id = AppId::from_uuid(uuid);
            match state.discovery_service.discover_by_id(&app_id) {
                Some(app) => (
                    StatusCode::OK,
                    Json(ApiResponse::ok(AppInfoResponse {
                        id: app.manifest.id.to_string(),
                        name: app.manifest.name.clone(),
                        version: app.manifest.version.to_string(),
                        description: app.manifest.description.clone(),
                        rating: app.rating,
                        downloads: app.download_count,
                        installed: app.installed,
                    })),
                )
                    .into_response(),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("App not found".to_string())),
                )
                    .into_response(),
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Invalid app ID format".to_string())),
        )
            .into_response(),
    }
}

/// GET /api/apps/discover - Discover apps with filters
pub async fn discover_apps(
    State(state): State<ApiState>,
    Query(query): Query<DiscoveryQuery>,
) -> Json<ApiResponse<Vec<AppInfoResponse>>> {
    let mut filter = DiscoveryFilter::new();

    if let Some(name) = query.name {
        filter = filter.with_name(name);
    }

    if let Some(category) = query.category {
        filter = filter.with_categories(vec![category]);
    }

    if let Some(min_rating) = query.min_rating {
        filter = filter.with_min_rating(min_rating);
    }

    let apps = state.discovery_service.discover(&filter);
    let limit = query.limit.unwrap_or(50).min(100);

    let response: Vec<AppInfoResponse> = apps
        .iter()
        .take(limit)
        .map(|app| AppInfoResponse {
            id: app.manifest.id.to_string(),
            name: app.manifest.name.clone(),
            version: app.manifest.version.to_string(),
            description: app.manifest.description.clone(),
            rating: app.rating,
            downloads: app.download_count,
            installed: app.installed,
        })
        .collect();

    Json(ApiResponse::ok(response))
}

/// GET /api/apps/search - Full-text search
pub async fn search_apps(
    State(state): State<ApiState>,
    Query(query): Query<SearchQuery>,
) -> Json<ApiResponse<Vec<AppInfoResponse>>> {
    let apps = state.discovery_service.discover_all();
    let results = SearchEngine::search(&apps, &query.q);
    let limit = query.limit.unwrap_or(50).min(100);

    let response: Vec<AppInfoResponse> = results
        .iter()
        .take(limit)
        .map(|result| {
            let app = &result.app;
            AppInfoResponse {
                id: app.manifest.id.to_string(),
                name: app.manifest.name.clone(),
                version: app.manifest.version.to_string(),
                description: app.manifest.description.clone(),
                rating: app.rating,
                downloads: app.download_count,
                installed: app.installed,
            }
        })
        .collect();

    Json(ApiResponse::ok(response))
}

/// POST /api/apps/:id/install - Install app
pub async fn install_app(
    State(_state): State<ApiState>,
    Path(app_id_str): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "status": "installing",
        "app_id": app_id_str,
        "progress": 0,
    })))
}

/// POST /api/apps/:id/uninstall - Uninstall app
pub async fn uninstall_app(
    State(_state): State<ApiState>,
    Path(app_id_str): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "status": "uninstalling",
        "app_id": app_id_str,
    })))
}

/// POST /api/apps/:id/start - Start app
pub async fn start_app(
    State(_state): State<ApiState>,
    Path(app_id_str): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "status": "started",
        "app_id": app_id_str,
    })))
}

/// POST /api/apps/:id/stop - Stop app
pub async fn stop_app(
    State(_state): State<ApiState>,
    Path(app_id_str): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "status": "stopped",
        "app_id": app_id_str,
    })))
}

/// GET /api/stats - System statistics
pub async fn get_stats(State(state): State<ApiState>) -> Json<ApiResponse<serde_json::Value>> {
    let all_apps = state.discovery_service.discover_all();
    let installed = all_apps.iter().filter(|a| a.installed).count();

    Json(ApiResponse::ok(serde_json::json!({
        "total_apps": all_apps.len(),
        "installed_apps": installed,
        "available_apps": all_apps.len() - installed,
        "avg_rating": all_apps.iter().map(|a| a.rating).sum::<f32>() / all_apps.len().max(1) as f32,
    })))
}

// ============================================================================
// Router Setup
// ============================================================================

pub fn create_router(state: ApiState) -> Router {
    use crate::handlers::{
        rate_app, add_review, get_reviews, get_ratings,
        get_trending, get_featured,
        list_modules, get_module, get_module_dependencies,
        list_installations, get_installation, update_app, get_versions,
        get_settings, update_settings, get_app_config, update_app_config,
    };

    Router::new()
        // Health & Status
        .route("/api/health", get(health_check))
        .route("/api/stats", get(get_stats))

        // App List & Discovery
        .route("/api/apps", get(list_apps))
        .route("/api/apps/:id", get(get_app))
        .route("/api/apps/discover", get(discover_apps))
        .route("/api/apps/search", get(search_apps))

        // App Operations
        .route("/api/apps/:id/install", post(install_app))
        .route("/api/apps/:id/uninstall", post(uninstall_app))
        .route("/api/apps/:id/start", post(start_app))
        .route("/api/apps/:id/stop", post(stop_app))

        // Marketplace (Reviews & Ratings)
        .route("/api/apps/:id/rate", post(rate_app))
        .route("/api/apps/:id/review", post(add_review))
        .route("/api/apps/:id/reviews", get(get_reviews))
        .route("/api/apps/:id/ratings", get(get_ratings))
        .route("/api/trending", get(get_trending))
        .route("/api/featured", get(get_featured))

        // Module Management
        .route("/api/modules", get(list_modules))
        .route("/api/modules/:id", get(get_module))
        .route("/api/modules/:id/dependencies", get(get_module_dependencies))

        // Installation Management
        .route("/api/installs", get(list_installations))
        .route("/api/installs/:id", get(get_installation))
        .route("/api/apps/:id/update", post(update_app))
        .route("/api/apps/:id/versions", get(get_versions))

        // Settings & Configuration
        .route("/api/settings", get(get_settings))
        .route("/api/settings", axum::routing::put(update_settings))
        .route("/api/apps/:id/config", get(get_app_config))
        .route("/api/apps/:id/config", axum::routing::put(update_app_config))

        .with_state(state)
}

// ============================================================================
// Server Startup
// ============================================================================

pub async fn start_server(
    addr: std::net::SocketAddr,
    state: ApiState,
) -> Result<(), Box<dyn std::error::Error>> {
    let router = create_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("API Server listening on {}", addr);

    axum::serve(listener, router).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok() {
        let response: ApiResponse<String> = ApiResponse::ok("test".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }
}
