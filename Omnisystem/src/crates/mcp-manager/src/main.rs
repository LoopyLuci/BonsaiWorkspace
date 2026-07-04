use axum::{
    Router,
    routing::{get, post, put, delete},
};
use mcp_manager::{
    AppState, McpServerConfig, McpClient, ExternalMcpServer, ToolEntry,
    server_config, clients, external_servers, tools,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let state = AppState {
        server_config: Arc::new(RwLock::new(McpServerConfig {
            host: "127.0.0.1".into(),
            port: 7780,
            auth_mode: "token".into(),
            max_clients: 100,
            rate_limit_per_minute: 60,
        })),
        connected_clients: Arc::new(RwLock::new(Vec::new())),
        external_servers: Arc::new(RwLock::new(vec![
            ExternalMcpServer {
                name: "Claude Desktop".into(),
                url: "https://api.anthropic.com".into(),
                status: "connected".into(),
                last_checked: "2026-06-03T00:00:00Z".into(),
            }
        ])),
        tool_registry: Arc::new(RwLock::new(vec![
            ToolEntry {
                name: "docker_list_containers".into(),
                description: "List all Docker containers".into(),
                category: "Infrastructure".into(),
                enabled: true,
                schema: serde_json::json!({}),
            },
            ToolEntry {
                name: "kdb_search".into(),
                description: "Search the Knowledge Database".into(),
                category: "Knowledge".into(),
                enabled: true,
                schema: serde_json::json!({}),
            },
        ])),
    };

    let app = Router::new()
        // Health check
        .route("/health", get(|| async { "OK" }))

        // Server config endpoints
        .route("/api/mcp/config", get(server_config::get_config))
        .route("/api/mcp/config", put(server_config::update_config))

        // Connected clients endpoints
        .route("/api/mcp/clients", get(clients::list_clients))
        .route("/api/mcp/clients/:id/revoke", post(clients::revoke_client))
        .route("/api/mcp/clients/:id/logs", get(clients::client_logs))

        // External servers endpoints
        .route("/api/mcp/servers", get(external_servers::list_external_servers))
        .route("/api/mcp/servers", post(external_servers::add_external_server))
        .route("/api/mcp/servers/:name/test", post(external_servers::test_connection))
        .route("/api/mcp/servers/:name", delete(external_servers::remove_server))

        // Tool registry endpoints
        .route("/api/mcp/tools", get(tools::list_tools))
        .route("/api/mcp/tools/:name/enable", post(tools::enable_tool))
        .route("/api/mcp/tools/:name/disable", post(tools::disable_tool))

        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state);

    println!("🔌 MCP Manager running on http://127.0.0.1:4201");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4201").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
