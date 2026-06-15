use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use uuid::Uuid;
use std::path::Path as StdPath;

use crate::client::{LauncherClient, AppMetadata, AppInstance, LaunchRequest, SystemStatus};

/// Web server for launcher daemon
pub struct LauncherServer {
    client: Arc<dyn LauncherClient>,
    addr: String,
}

impl LauncherServer {
    pub fn new(client: Arc<dyn LauncherClient>, addr: String) -> Self {
        Self { client, addr }
    }

    pub async fn start(self) -> Result<()> {
        use tower_http::services::ServeDir;

        let www_path = "C:\\Launcher\\www";

        let api_router = Router::new()
            .route("/api/health", get(health_check))
            .route("/api/apps", get(list_apps))
            .route("/api/apps/:id", get(get_app))
            .route("/api/search", get(search_apps))
            .route("/api/launch", post(launch_app))
            .route("/api/instances", get(list_instances))
            .route("/api/instances/:id/terminate", post(terminate_app))
            .route("/api/status", get(get_status))
            .with_state(self.client.clone());

        // Serve static files from C:\Launcher\www if directory exists
        let app = if StdPath::new(www_path).exists() {
            api_router.fallback_service(ServeDir::new(www_path))
        } else {
            api_router
        };

        let listener = TcpListener::bind(&self.addr).await?;
        tracing::info!("Launcher server listening on {}", self.addr);
        if StdPath::new(www_path).exists() {
            tracing::info!("Serving static files from: {}", www_path);
        }

        axum::serve(listener, app).await?;
        Ok(())
    }
}

// Handlers
async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "version": "1.0.0"
    }))
}

async fn list_apps(
    State(client): State<Arc<dyn LauncherClient>>,
) -> Result<Json<Vec<AppMetadata>>, (StatusCode, String)> {
    client
        .list_apps()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_app(
    State(client): State<Arc<dyn LauncherClient>>,
    Path(app_id): Path<String>,
) -> Result<Json<Option<AppMetadata>>, (StatusCode, String)> {
    client
        .get_app(&app_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: String,
}

async fn search_apps(
    State(client): State<Arc<dyn LauncherClient>>,
    Query(SearchQuery { q }): Query<SearchQuery>,
) -> Result<Json<Vec<AppMetadata>>, (StatusCode, String)> {
    client
        .search_apps(&q)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Deserialize)]
pub struct LaunchRequestBody {
    pub app_id: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: String,
}

fn default_priority() -> String {
    "normal".to_string()
}

#[derive(Serialize)]
pub struct LaunchResponseBody {
    pub instance_id: Uuid,
    pub status: String,
}

async fn launch_app(
    State(client): State<Arc<dyn LauncherClient>>,
    Json(body): Json<LaunchRequestBody>,
) -> Result<Json<LaunchResponseBody>, (StatusCode, String)> {
    let request = LaunchRequest {
        app_id: body.app_id,
        args: body.args,
        priority: body.priority,
    };

    client
        .launch_app(request)
        .await
        .map(|resp| {
            Json(LaunchResponseBody {
                instance_id: resp.instance_id,
                status: resp.status,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn list_instances(
    State(client): State<Arc<dyn LauncherClient>>,
) -> Result<Json<Vec<AppInstance>>, (StatusCode, String)> {
    client
        .list_instances()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn terminate_app(
    State(client): State<Arc<dyn LauncherClient>>,
    Path(instance_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    client
        .terminate_app(&instance_id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_status(
    State(client): State<Arc<dyn LauncherClient>>,
) -> Result<Json<SystemStatus>, (StatusCode, String)> {
    client
        .get_system_status()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query() {
        let json = r#"{"q": "test"}"#;
        let query: SearchQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.q, "test");
    }

    #[test]
    fn test_launch_request_body() {
        let json = r#"{"app_id": "app1", "args": ["arg1"], "priority": "high"}"#;
        let body: LaunchRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.app_id, "app1");
        assert_eq!(body.priority, "high");
    }
}
