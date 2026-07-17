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
/// Superseded by `tool_registry` (which is what `server.rs` actually
/// dispatches through); kept available for direct use but not re-exported at
/// the crate root to avoid clashing with `tool_registry::ToolDefinition`.
pub mod mcp_tools;
/// Superseded by `uacs` (which has the same `HeadlessConfig`/mode/event
/// shapes plus HITL support); kept available but not re-exported at the
/// crate root to avoid clashing with `uacs::HeadlessConfig`.
pub mod visualiser;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
	pub name: String,
	pub description: String,
	pub input_schema: serde_json::Value,
}

// Re-export tool registry for convenient access
pub use tool_registry::McpToolRegistry;
