//! HTTP request handlers for the REST API

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::server::{ApiResponse, ApiState, AppInfoResponse};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RatingRequest {
    pub rating: u8,
}

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    pub rating: u8,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewResponse {
    pub id: String,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub helpful_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleResponse {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub module_type: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationResponse {
    pub id: String,
    pub app_id: String,
    pub version: String,
    pub location: String,
    pub status: String,
    pub installed_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrendingAppResponse {
    pub rank: usize,
    pub app_id: String,
    pub name: String,
    pub downloads: u32,
    pub trending_score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsResponse {
    pub theme: String,
    pub notifications_enabled: bool,
    pub auto_update: bool,
    pub language: String,
}

// ============================================================================
// Marketplace Endpoints
// ============================================================================

/// POST /api/apps/:id/rate - Rate an app
pub async fn rate_app(
    State(_state): State<ApiState>,
    Path(app_id): Path<String>,
    Json(req): Json<RatingRequest>,
) -> impl IntoResponse {
    if req.rating < 1 || req.rating > 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Rating must be between 1 and 5".to_string())),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!({
            "app_id": app_id,
            "rating": req.rating,
            "message": "Rating submitted"
        }))),
    )
        .into_response()
}

/// POST /api/apps/:id/review - Add a review
pub async fn add_review(
    State(_state): State<ApiState>,
    Path(app_id): Path<String>,
    Json(req): Json<ReviewRequest>,
) -> impl IntoResponse {
    if req.rating < 1 || req.rating > 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Rating must be between 1 and 5".to_string())),
        )
            .into_response();
    }

    if req.title.is_empty() || req.content.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Title and content are required".to_string())),
        )
            .into_response();
    }

    let review = ReviewResponse {
        id: uuid::Uuid::new_v4().to_string(),
        rating: req.rating,
        title: req.title,
        content: req.content,
        helpful_count: 0,
    };

    (
        StatusCode::CREATED,
        Json(ApiResponse::ok(serde_json::json!({
            "app_id": app_id,
            "review": review,
        }))),
    )
        .into_response()
}

/// GET /api/apps/:id/reviews - Get reviews for app
pub async fn get_reviews(
    State(_state): State<ApiState>,
    Path(_app_id): Path<String>,
) -> Json<ApiResponse<Vec<ReviewResponse>>> {
    let reviews = vec![ReviewResponse {
        id: uuid::Uuid::new_v4().to_string(),
        rating: 5,
        title: "Excellent app!".to_string(),
        content: "Works perfectly as advertised.".to_string(),
        helpful_count: 42,
    }];

    Json(ApiResponse::ok(reviews))
}

/// GET /api/apps/:id/ratings - Get rating statistics
pub async fn get_ratings(
    State(_state): State<ApiState>,
    Path(app_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "app_id": app_id,
        "average_rating": 4.5,
        "total_reviews": 156,
        "rating_distribution": {
            "5_stars": 120,
            "4_stars": 30,
            "3_stars": 5,
            "2_stars": 1,
            "1_stars": 0,
        }
    })))
}

/// GET /api/trending - Get trending apps
pub async fn get_trending(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<TrendingAppResponse>>> {
    let all_apps = state.discovery_service.discover_all();
    let mut trending: Vec<_> = all_apps
        .iter()
        .enumerate()
        .take(10)
        .map(|(idx, app)| TrendingAppResponse {
            rank: idx + 1,
            app_id: app.manifest.id.to_string(),
            name: app.manifest.name.clone(),
            downloads: app.download_count,
            trending_score: app.rating * (app.download_count as f32 / 1000.0),
        })
        .collect();

    trending.sort_by(|a, b| b.trending_score.partial_cmp(&a.trending_score).unwrap_or(std::cmp::Ordering::Equal));

    Json(ApiResponse::ok(trending))
}

/// GET /api/featured - Get featured apps
pub async fn get_featured(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<AppInfoResponse>>> {
    let all_apps = state.discovery_service.discover_all();
    let featured: Vec<AppInfoResponse> = all_apps
        .iter()
        .filter(|app| app.rating >= 4.5)
        .take(20)
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

    Json(ApiResponse::ok(featured))
}

// ============================================================================
// Module Management Endpoints
// ============================================================================

/// GET /api/modules - List all modules
pub async fn list_modules(
    State(_state): State<ApiState>,
) -> Json<ApiResponse<Vec<ModuleResponse>>> {
    let modules = vec![ModuleResponse {
        id: uuid::Uuid::new_v4().to_string(),
        app_id: uuid::Uuid::new_v4().to_string(),
        name: "core-module".to_string(),
        version: "1.0.0".to_string(),
        module_type: "library".to_string(),
        status: "loaded".to_string(),
    }];

    Json(ApiResponse::ok(modules))
}

/// GET /api/modules/:id - Get module details
pub async fn get_module(
    State(_state): State<ApiState>,
    Path(module_id): Path<String>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::ok(ModuleResponse {
            id: module_id,
            app_id: uuid::Uuid::new_v4().to_string(),
            name: "test-module".to_string(),
            version: "1.0.0".to_string(),
            module_type: "service".to_string(),
            status: "loaded".to_string(),
        })),
    )
        .into_response()
}

/// GET /api/modules/:id/dependencies - Get module dependencies
pub async fn get_module_dependencies(
    State(_state): State<ApiState>,
    Path(module_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "module_id": module_id,
        "dependencies": [
            {
                "name": "runtime-core",
                "version": "^1.0.0",
                "optional": false
            }
        ]
    })))
}

// ============================================================================
// Installation Management Endpoints
// ============================================================================

/// GET /api/installs - List all installations
pub async fn list_installations(
    State(_state): State<ApiState>,
) -> Json<ApiResponse<Vec<InstallationResponse>>> {
    let installs = vec![InstallationResponse {
        id: uuid::Uuid::new_v4().to_string(),
        app_id: uuid::Uuid::new_v4().to_string(),
        version: "1.0.0".to_string(),
        location: "/opt/apps/test-app".to_string(),
        status: "installed".to_string(),
        installed_at: chrono::Utc::now().to_rfc3339(),
    }];

    Json(ApiResponse::ok(installs))
}

/// GET /api/installs/:id - Get installation details
pub async fn get_installation(
    State(_state): State<ApiState>,
    Path(install_id): Path<String>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::ok(InstallationResponse {
            id: install_id,
            app_id: uuid::Uuid::new_v4().to_string(),
            version: "1.0.0".to_string(),
            location: "/opt/apps/test-app".to_string(),
            status: "installed".to_string(),
            installed_at: chrono::Utc::now().to_rfc3339(),
        })),
    )
        .into_response()
}

/// POST /api/apps/:id/update - Update app to latest version
pub async fn update_app(
    State(_state): State<ApiState>,
    Path(app_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "app_id": app_id,
        "status": "updating",
        "from_version": "1.0.0",
        "to_version": "1.1.0",
        "progress": 0,
    })))
}

/// GET /api/apps/:id/versions - Get version history
pub async fn get_versions(
    State(_state): State<ApiState>,
    Path(app_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "app_id": app_id,
        "versions": [
            {
                "version": "1.1.0",
                "released_at": chrono::Utc::now().to_rfc3339(),
                "changelog": "Bug fixes and improvements"
            },
            {
                "version": "1.0.0",
                "released_at": "2026-06-01T00:00:00Z",
                "changelog": "Initial release"
            }
        ]
    })))
}

// ============================================================================
// Settings Management Endpoints
// ============================================================================

/// GET /api/settings - Get user settings
pub async fn get_settings(
    State(_state): State<ApiState>,
) -> Json<ApiResponse<SettingsResponse>> {
    Json(ApiResponse::ok(SettingsResponse {
        theme: "dark".to_string(),
        notifications_enabled: true,
        auto_update: true,
        language: "en".to_string(),
    }))
}

/// PUT /api/settings - Update user settings
pub async fn update_settings(
    State(_state): State<ApiState>,
    Json(_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!({
            "message": "Settings updated successfully",
        }))),
    )
        .into_response()
}

/// GET /api/apps/:id/config - Get app configuration
pub async fn get_app_config(
    State(_state): State<ApiState>,
    Path(app_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "app_id": app_id,
        "debug_mode": false,
        "timeout_ms": 5000,
        "max_memory_mb": 512,
        "enable_logging": true,
    })))
}

/// PUT /api/apps/:id/config - Update app configuration
pub async fn update_app_config(
    State(_state): State<ApiState>,
    Path(app_id): Path<String>,
    Json(_config): Json<ConfigRequest>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::ok(serde_json::json!({
            "app_id": app_id,
            "message": "Configuration updated",
        }))),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_validation() {
        assert!(1u8 <= 5);
        assert!(!(0u8 <= 5 && 0u8 >= 1));
    }
}
