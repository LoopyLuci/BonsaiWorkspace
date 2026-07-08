pub mod server;
pub mod tools;
pub mod auth;
pub mod bridge;
pub mod uacs;
pub mod mobile_session;
pub mod bti_commands;
pub mod bug_hunt_tools;
pub mod scan_rules;
pub mod lint_tools;
pub mod tool_registry;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
	pub name: String,
	pub description: String,
	pub input_schema: serde_json::Value,
}

// Re-export tool registry for convenient access
pub use tool_registry::McpToolRegistry;
