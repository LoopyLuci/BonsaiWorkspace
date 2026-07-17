//! CLI — list every MCP tool registered by `McpToolRegistry`.
//!
//! For the full UACS agent-control server, run the `uacs` binary (`src/main.rs`).

use mcp_server::McpToolRegistry;

fn main() {
    let registry = McpToolRegistry::new();
    let mut tools = registry.list_tools();
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    println!("Registered MCP tools ({}):", tools.len());
    for tool in tools {
        println!("  {} — {}", tool.name, tool.description);
    }
}
