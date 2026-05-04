/// MCP (Model Context Protocol) bridge — stdio transport.
///
/// Each configured MCP server is launched as a subprocess. After a successful
/// JSON-RPC `initialize` handshake the server's tool list is fetched and each
/// tool is wrapped in an `McpToolAdapter` that implements the `Tool` trait.
use std::sync::Arc;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, RwLock};

use crate::tool_core::{
    SideEffectProfile, Tool, ToolContext, ToolError, ToolOutput, ToolPolicyHint,
    ToolRegistry, ToolResult,
};

// ── Server config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub id:        String,
    pub name:      String,
    pub command:   String,
    pub args:      Vec<String>,
    pub enabled:   bool,
    pub namespace: String,
}

// ── MCP tool definition (from tools/list) ────────────────────────────────────

pub struct McpToolDef {
    pub name:         String,
    pub description:  String,
    pub input_schema: Value,
}

// ── JSON-RPC / MCP connection (one subprocess) ────────────────────────────────

pub struct McpConnection {
    config:  McpServerConfig,
    #[allow(dead_code)]
    child:   tokio::process::Child,
    stdin:   tokio::process::ChildStdin,
    stdout:  BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

impl McpConnection {
    pub async fn connect(config: McpServerConfig) -> Result<Self, String> {
        use std::process::Stdio;

        let mut child = {
            let mut c = tokio::process::Command::new(&config.command);
            c.args(&config.args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            }
            c.spawn()
                .map_err(|e| format!("mcp spawn '{}': {e}", config.command))?
        };

        let stdin  = child.stdin.take()
            .ok_or_else(|| "mcp: no stdin handle".to_string())?;
        let stdout = BufReader::new(
            child.stdout.take()
                .ok_or_else(|| "mcp: no stdout handle".to_string())?
        );

        let mut conn = McpConnection { config, child, stdin, stdout, next_id: 1 };

        // initialize handshake
        conn.send(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "bonsai", "version": "0.1.0" }
            }),
        ).await?;

        // send initialized notification (no id)
        let notif = format!(
            "{}\n",
            serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            })).unwrap()
        );
        conn.stdin.write_all(notif.as_bytes()).await
            .map_err(|e| format!("mcp write initialized: {e}"))?;

        Ok(conn)
    }

    async fn send(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        let req = format!(
            "{}\n",
            serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            })).unwrap()
        );
        self.stdin.write_all(req.as_bytes()).await
            .map_err(|e| format!("mcp write: {e}"))?;
        self.stdin.flush().await
            .map_err(|e| format!("mcp flush: {e}"))?;

        let mut line = String::new();
        self.stdout.read_line(&mut line).await
            .map_err(|e| format!("mcp read: {e}"))?;

        let resp: Value = serde_json::from_str(line.trim())
            .map_err(|e| format!("mcp parse: {e} — raw: {line}"))?;

        if let Some(err) = resp.get("error") {
            return Err(format!("mcp error: {err}"));
        }
        Ok(resp["result"].clone())
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpToolDef>, String> {
        let result = self.send("tools/list", json!({})).await?;
        let tools = result["tools"].as_array()
            .ok_or_else(|| "mcp tools/list: missing 'tools' array".to_string())?;

        Ok(tools.iter().map(|t| McpToolDef {
            name:         t["name"].as_str().unwrap_or("").to_string(),
            description:  t["description"].as_str().unwrap_or("").to_string(),
            input_schema: t.get("inputSchema").cloned().unwrap_or_else(|| json!({"type":"object","properties":{}})),
        }).collect())
    }

    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value, String> {
        let result = self.send(
            "tools/call",
            json!({ "name": name, "arguments": args }),
        ).await?;

        // MCP returns content array; collapse to a single value
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            let text: String = content.iter()
                .filter_map(|c| c["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n");
            Ok(json!({ "content": text }))
        } else {
            Ok(result)
        }
    }
}

// ── McpToolAdapter — wraps one MCP tool as a Tool impl ───────────────────────

pub struct McpToolAdapter {
    tool_name:   &'static str,
    original:    String,
    description: &'static str,
    schema:      Value,
    connection:  Arc<Mutex<McpConnection>>,
}

#[async_trait::async_trait]
impl Tool for McpToolAdapter {
    fn name(&self)         -> &'static str { self.tool_name }
    fn description(&self)  -> &'static str { self.description }
    fn schema(&self)       -> Value        { self.schema.clone() }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::External }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::external() }
    fn tags(&self)         -> &'static [&'static str] { &["mcp", "external"] }

    async fn execute(&self, args: &Value, ctx: &ToolContext) -> ToolResult {
        if ctx.is_cancelled() {
            return Err(ToolError::Internal { message: "cancelled".into() });
        }
        let mut conn = self.connection.lock().await;
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            conn.call_tool(&self.original, args.clone()),
        )
        .await
        .map_err(|_| ToolError::Timeout { duration_ms: 60_000 })?
        .map_err(|e| ToolError::Transient { message: e, retry_after_ms: None })?;

        Ok(ToolOutput::Complete(result))
    }
}

// ── McpManager — lifecycle for all configured MCP servers ────────────────────

pub struct McpManager {
    configs: RwLock<Vec<McpServerConfig>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self { configs: RwLock::new(Vec::new()) }
    }

    pub async fn load_configs(&self, configs: Vec<McpServerConfig>) {
        *self.configs.write().await = configs;
    }

    /// Spawn every enabled server, list its tools, and register adapters.
    /// Returns names of servers that connected successfully.
    pub async fn connect_all_into_registry(
        &self,
        registry: &mut ToolRegistry,
        allowed_commands: &[String],
    ) -> Vec<String> {
        let configs = self.configs.read().await.clone();
        let mut connected = Vec::new();

        if allowed_commands.is_empty() {
            tracing::warn!("[mcp] mcp_allowed_commands is empty; allowing all configured MCP commands");
        }

        for cfg in configs {
            if !cfg.enabled { continue; }

            if !allowed_commands.is_empty()
                && !allowed_commands.iter().any(|allowed| {
                    if cfg!(windows) {
                        allowed.eq_ignore_ascii_case(&cfg.command)
                    } else {
                        allowed == &cfg.command
                    }
                })
            {
                tracing::warn!(
                    server=%cfg.name,
                    command=%cfg.command,
                    "[mcp] skipping server: command not in mcp_allowed_commands"
                );
                continue;
            }

            let name = cfg.name.clone();
            let namespace = cfg.namespace.clone();

            match McpConnection::connect(cfg).await {
                Err(e) => {
                    tracing::error!(server=%name, error=%e, "[mcp] server connect failed");
                }
                Ok(mut conn) => {
                    match conn.list_tools().await {
                        Err(e) => {
                            tracing::error!(server=%name, error=%e, "[mcp] server list_tools failed");
                        }
                        Ok(tool_defs) => {
                            let shared = Arc::new(Mutex::new(conn));
                            for def in tool_defs {
                                let qualified = format!("{namespace}__{}", def.name);
                                let tool_name  = crate::tool_core::intern_str(qualified);
                                let description = crate::tool_core::intern_str(def.description);
                                let adapter = McpToolAdapter {
                                    tool_name,
                                    original: def.name,
                                    description,
                                    schema: def.input_schema,
                                    connection: shared.clone(),
                                };
                                registry.register(adapter);
                            }
                            connected.push(name);
                        }
                    }
                }
            }
        }

        connected
    }

    pub async fn disconnect_server(&self, server_id: &str) {
        // Connections are owned by their Arc<Mutex<McpConnection>>; drop is
        // handled when all McpToolAdapter arcs go out of scope (registry clear).
        // This method is a placeholder for future per-server teardown.
        tracing::info!(server_id=%server_id, "[mcp] disconnect_server called (adapters will drop with registry)");
    }
}
