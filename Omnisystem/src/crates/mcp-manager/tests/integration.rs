use axum::extract::{Path, State};
use axum::Json;
use mcp_manager::{
    clients, external_servers, server_config, tools, AppState, McpClient, McpServerConfig,
    ToolEntry,
};

fn default_config() -> McpServerConfig {
    McpServerConfig {
        host: "127.0.0.1".to_string(),
        port: 7780,
        auth_mode: "token".to_string(),
        max_clients: 100,
        rate_limit_per_minute: 60,
    }
}

#[tokio::test]
async fn test_get_and_update_config() {
    let state = AppState::new(default_config());

    let Json(config) = server_config::get_config(State(state.clone())).await;
    assert_eq!(config.port, 7780);

    let Json(_) = server_config::update_config(
        State(state.clone()),
        Json(server_config::UpdateConfigRequest {
            host: None,
            port: Some(9000),
            auth_mode: None,
            max_clients: None,
            rate_limit_per_minute: None,
        }),
    )
    .await;

    let Json(updated) = server_config::get_config(State(state)).await;
    assert_eq!(updated.port, 9000);
    // Untouched fields survive a partial update.
    assert_eq!(updated.auth_mode, "token");
}

#[tokio::test]
async fn test_add_list_and_remove_external_server() {
    let state = AppState::new(default_config());

    let _ = external_servers::add_external_server(
        State(state.clone()),
        Json(external_servers::AddServerRequest {
            name: "test-server".to_string(),
            url: "https://example.com".to_string(),
        }),
    )
    .await;

    let Json(servers) = external_servers::list_external_servers(State(state.clone())).await;
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "test-server");

    let _ =
        external_servers::remove_server(State(state.clone()), Path("test-server".to_string()))
            .await;

    let Json(servers) = external_servers::list_external_servers(State(state)).await;
    assert!(servers.is_empty());
}

#[tokio::test]
async fn test_enable_and_disable_tool() {
    let state = AppState::new(default_config());
    state.tool_registry.write().await.push(ToolEntry {
        name: "search".to_string(),
        description: "Search things".to_string(),
        category: "General".to_string(),
        enabled: false,
        schema: serde_json::json!({}),
    });

    let _ = tools::enable_tool(State(state.clone()), Path("search".to_string())).await;
    let Json(tool_list) = tools::list_tools(State(state.clone())).await;
    assert!(tool_list[0].enabled);

    let _ = tools::disable_tool(State(state.clone()), Path("search".to_string())).await;
    let Json(tool_list) = tools::list_tools(State(state)).await;
    assert!(!tool_list[0].enabled);
}

#[tokio::test]
async fn test_revoke_unknown_client_reports_error() {
    let state = AppState::new(default_config());
    let Json(result) = clients::revoke_client(State(state), Path("nope".to_string())).await;
    assert!(result.get("error").is_some());
}

#[tokio::test]
async fn test_revoke_known_client_marks_revoked() {
    let state = AppState::new(default_config());
    state.connected_clients.write().await.push(McpClient {
        client_id: "c1".to_string(),
        name: "editor".to_string(),
        status: "connected".to_string(),
        connected_at: "2026-01-01T00:00:00Z".to_string(),
    });

    let Json(result) = clients::revoke_client(State(state.clone()), Path("c1".to_string())).await;
    assert_eq!(result["status"], "revoked");

    let Json(client_list) = clients::list_clients(State(state)).await;
    assert_eq!(client_list[0].status, "revoked");
}
