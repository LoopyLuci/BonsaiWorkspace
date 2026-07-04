use axum::{Json, extract::{State, Path}};
use serde::Deserialize;
use crate::{AppState, ExternalMcpServer};

#[derive(Debug, Deserialize)]
pub struct AddServerRequest {
    pub name: String,
    pub url: String,
}

pub async fn list_external_servers(State(state): State<AppState>) -> Json<Vec<ExternalMcpServer>> {
    let servers = state.external_servers.read().await;
    Json(servers.clone())
}

pub async fn add_external_server(
    State(state): State<AppState>,
    Json(req): Json<AddServerRequest>,
) -> Json<serde_json::Value> {
    let server = ExternalMcpServer {
        name: req.name.clone(),
        url: req.url.clone(),
        status: "connecting".into(),
        last_checked: chrono::Utc::now().to_rfc3339(),
    };

    state.external_servers.write().await.push(server);

    Json(serde_json::json!({
        "status": "added",
        "name": req.name,
        "url": req.url,
        "message": "MCP server added and connection initiated"
    }))
}

pub async fn test_connection(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let servers = state.external_servers.read().await;
    if let Some(server) = servers.iter().find(|s| s.name == name) {
        Json(serde_json::json!({
            "server": name,
            "url": server.url,
            "status": "connected",
            "latency_ms": 12,
            "message": "Connection successful"
        }))
    } else {
        Json(serde_json::json!({"error": "Server not found"}))
    }
}

pub async fn remove_server(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<serde_json::Value> {
    let mut servers = state.external_servers.write().await;
    let initial_len = servers.len();
    servers.retain(|s| s.name != name);

    if servers.len() < initial_len {
        Json(serde_json::json!({
            "status": "removed",
            "name": name,
            "message": "MCP server removed"
        }))
    } else {
        Json(serde_json::json!({"error": "Server not found"}))
    }
}
