//! Shared types for the MCP manager server: connected clients, the
//! server's own config, registered external MCP servers, and the tool
//! registry -- plus the [`AppState`] axum handlers share access to.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for this MCP manager's own server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub host: String,
    pub port: u16,
    pub auth_mode: String,
    pub max_clients: u32,
    pub rate_limit_per_minute: u32,
}

/// A client (e.g. an editor or agent) currently connected to this MCP
/// server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClient {
    pub client_id: String,
    pub name: String,
    pub status: String,
    pub connected_at: String,
}

/// An external MCP server this manager proxies/monitors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMcpServer {
    pub name: String,
    pub url: String,
    pub status: String,
    pub last_checked: String,
}

/// A tool exposed through the MCP tool registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    pub name: String,
    pub description: String,
    pub category: String,
    pub enabled: bool,
    pub schema: serde_json::Value,
}

/// Shared application state passed to every axum handler.
#[derive(Clone)]
pub struct AppState {
    pub server_config: Arc<RwLock<McpServerConfig>>,
    pub connected_clients: Arc<RwLock<Vec<McpClient>>>,
    pub external_servers: Arc<RwLock<Vec<ExternalMcpServer>>>,
    pub tool_registry: Arc<RwLock<Vec<ToolEntry>>>,
}

impl AppState {
    pub fn new(server_config: McpServerConfig) -> Self {
        Self {
            server_config: Arc::new(RwLock::new(server_config)),
            connected_clients: Arc::new(RwLock::new(Vec::new())),
            external_servers: Arc::new(RwLock::new(Vec::new())),
            tool_registry: Arc::new(RwLock::new(Vec::new())),
        }
    }
}
