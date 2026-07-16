//! MCP manager: an axum-based HTTP API for managing an MCP (Model Context
//! Protocol) server's own configuration, connected clients, external MCP
//! server registrations, and the local tool registry.

pub mod clients;
pub mod error;
pub mod external_servers;
pub mod server_config;
pub mod tools;
pub mod types;

pub use error::{Error, Result};
pub use types::{AppState, ExternalMcpServer, McpClient, McpServerConfig, ToolEntry};
