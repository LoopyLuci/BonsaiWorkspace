use axum::{Json, extract::State};
use serde::Deserialize;
use crate::{AppState, McpServerConfig};

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub auth_mode: Option<String>,
    pub max_clients: Option<u32>,
    pub rate_limit_per_minute: Option<u32>,
}

pub async fn get_config(State(state): State<AppState>) -> Json<McpServerConfig> {
    Json(state.server_config.read().await.clone())
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Json<serde_json::Value> {
    let mut config = state.server_config.write().await;

    if let Some(host) = req.host {
        config.host = host;
    }
    if let Some(port) = req.port {
        config.port = port;
    }
    if let Some(auth_mode) = req.auth_mode {
        config.auth_mode = auth_mode;
    }
    if let Some(max_clients) = req.max_clients {
        config.max_clients = max_clients;
    }
    if let Some(rate_limit) = req.rate_limit_per_minute {
        config.rate_limit_per_minute = rate_limit;
    }

    Json(serde_json::json!({
        "status": "updated",
        "message": "MCP server configuration updated. Restart required for changes to take effect."
    }))
}
