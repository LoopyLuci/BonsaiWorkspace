//! mcp-manager CLI: exercises the axum handlers directly (in-process, no
//! HTTP server) to register an external MCP server, register a tool,
//! connect a client, then list everything back out.

use axum::extract::{Path, State};
use axum::Json;
use mcp_manager::{clients, external_servers, tools, AppState, McpClient, McpServerConfig, ToolEntry};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState::new(McpServerConfig {
        host: "127.0.0.1".to_string(),
        port: 7780,
        auth_mode: "token".to_string(),
        max_clients: 100,
        rate_limit_per_minute: 60,
    });

    let Json(add_result) = external_servers::add_external_server(
        State(state.clone()),
        Json(external_servers::AddServerRequest {
            name: "Claude Desktop".to_string(),
            url: "https://api.anthropic.com".to_string(),
        }),
    )
    .await;
    println!("Add-server result: {}", add_result);

    state.tool_registry.write().await.push(ToolEntry {
        name: "kdb_search".to_string(),
        description: "Search the Knowledge Database".to_string(),
        category: "Knowledge".to_string(),
        enabled: true,
        schema: serde_json::json!({}),
    });

    state.connected_clients.write().await.push(McpClient {
        client_id: "client-1".to_string(),
        name: "demo-editor".to_string(),
        status: "connected".to_string(),
        connected_at: chrono::Utc::now().to_rfc3339(),
    });

    let Json(servers) = external_servers::list_external_servers(State(state.clone())).await;
    println!("External servers: {}", servers.len());

    let Json(tool_list) = tools::list_tools(State(state.clone())).await;
    println!("Registered tools: {}", tool_list.len());

    let Json(client_list) = clients::list_clients(State(state.clone())).await;
    println!("Connected clients: {}", client_list.len());

    let revoked = clients::revoke_client(State(state.clone()), Path("client-1".to_string())).await;
    println!("Revoke result: {}", revoked.0);

    let enabled = tools::enable_tool(State(state), Path("kdb_search".to_string())).await;
    println!("Enable-tool result: {}", enabled.0);

    Ok(())
}
