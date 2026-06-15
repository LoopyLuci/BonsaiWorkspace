use axum::{Json, extract::{State, Path}};
use crate::{AppState, McpClient};

pub async fn list_clients(State(state): State<AppState>) -> Json<Vec<McpClient>> {
    let clients = state.connected_clients.read().await;
    Json(clients.clone())
}

pub async fn revoke_client(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Json<serde_json::Value> {
    let mut clients = state.connected_clients.write().await;
    if let Some(client) = clients.iter_mut().find(|c| c.client_id == client_id) {
        client.status = "revoked".into();
        Json(serde_json::json!({
            "status": "revoked",
            "client_id": client_id,
            "message": "Client access revoked"
        }))
    } else {
        Json(serde_json::json!({"error": "Client not found"}))
    }
}

pub async fn client_logs(
    State(state): State<AppState>,
    Path(client_id): Path<String>,
) -> Json<serde_json::Value> {
    let clients = state.connected_clients.read().await;
    if let Some(_client) = clients.iter().find(|c| c.client_id == client_id) {
        Json(serde_json::json!({
            "client_id": client_id,
            "logs": vec![
                "2026-06-03T00:00:00Z - Connected",
                "2026-06-03T00:05:00Z - Called docker_list_containers",
                "2026-06-03T00:10:00Z - Called kdb_search"
            ]
        }))
    } else {
        Json(serde_json::json!({"error": "Client not found"}))
    }
}
