use crate::AppState;
// MCP (Model Context Protocol) server — Bonsai Workspace as a universal tool backend.
//
// Implements the MCP 2025-03-26 spec over Streamable HTTP transport (JSON-RPC 2.0).
// Any MCP-compatible client (Claude Desktop, Cursor, VS Code Continue, custom agents)
// can connect to this server and invoke any Bonsai tool, read workspace resources,
// or fetch pre-built prompt templates.
//
// ## Endpoints
//   POST /mcp          — JSON-RPC 2.0 request/response
//   GET  /mcp/sse      — SSE stream for server-initiated messages (optional)
//   GET  /mcp/health   — health check
//
// ## Supported JSON-RPC methods
//   initialize          — capability negotiation
//   tools/list          — list all registered tools with input schemas
//   tools/call          — invoke a tool by name
//   resources/list      — list available resources (workspace, memory)
//   resources/read      — read a resource by URI
//   prompts/list        — list built-in prompt templates
//   prompts/get         — get a rendered prompt template

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::tool_core::ToolRegistry;

// ── JSON-RPC 2.0 types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }
    fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

// ── MCP capability manifest ───────────────────────────────────────────────────

const MCP_VERSION: &str = "2025-03-26";
const SERVER_NAME: &str = "bonsai-workspace";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

fn server_capabilities() -> Value {
    json!({
        "tools": { "listChanged": true },
        "resources": { "subscribe": false, "listChanged": false },
        "prompts": { "listChanged": false },
        "logging": {}
    })
}

// ── Built-in prompt templates ─────────────────────────────────────────────────

fn builtin_prompts() -> Vec<Value> {
    vec![
        json!({
            "name": "code_review",
            "description": "Review a file for correctness, style, and security issues.",
            "arguments": [
                { "name": "file_path", "description": "Absolute path to the file to review.", "required": true }
            ]
        }),
        json!({
            "name": "explain_codebase",
            "description": "Summarise what a project does by walking its file tree.",
            "arguments": [
                { "name": "root", "description": "Workspace root path.", "required": true }
            ]
        }),
        json!({
            "name": "fix_test_failure",
            "description": "Diagnose a failing test and suggest a fix.",
            "arguments": [
                { "name": "error_output", "description": "The stderr / test output.", "required": true }
            ]
        }),
        json!({
            "name": "generate_music",
            "description": "Compose an original music track from a text description.",
            "arguments": [
                { "name": "prompt", "description": "Description of the desired music.", "required": true },
                { "name": "duration", "description": "Length in seconds (1–60).", "required": false }
            ]
        }),
    ]
}

fn render_prompt(name: &str, args: &Value) -> Option<Value> {
    let get = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("");
    let messages = match name {
        "code_review" => vec![json!({
            "role": "user",
            "content": {
                "type": "text",
                "text": format!(
                    "Please review the file at `{}` for:\n\
                     1. Correctness and logic errors\n\
                     2. Security vulnerabilities\n\
                     3. Style and naming consistency\n\
                     4. Performance issues\n\
                     Use the `read_file` tool to read it first.",
                    get("file_path")
                )
            }
        })],
        "explain_codebase" => vec![json!({
            "role": "user",
            "content": {
                "type": "text",
                "text": format!(
                    "Use `list_all_files` to explore `{}`, read key files, and give a \
                     concise explanation of what this project does, its architecture, and \
                     the main technologies it uses.",
                    get("root")
                )
            }
        })],
        "fix_test_failure" => vec![json!({
            "role": "user",
            "content": {
                "type": "text",
                "text": format!(
                    "The following test output indicates a failure. Diagnose the root cause \
                     and suggest a precise fix:\n\n```\n{}\n```",
                    get("error_output")
                )
            }
        })],
        "generate_music" => vec![json!({
            "role": "user",
            "content": {
                "type": "text",
                "text": format!(
                    "Generate a music track: {}{}\nUse the `generate_music` tool.",
                    get("prompt"),
                    {
                        let d = get("duration");
                        if d.is_empty() { String::new() } else { format!(" ({}s)", d) }
                    }
                )
            }
        })],
        _ => return None,
    };
    Some(json!({ "description": format!("Prompt: {name}"), "messages": messages }))
}

// ── Shared MCP state ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct McpState {
    /// Shared assistant tool registry — source of truth for tools/list and tools/call.
    registry: Arc<RwLock<ToolRegistry>>,
    /// Workspace root for resource reads.
    workspace_root: Arc<Option<String>>,
    /// Bonsai memory file path for bonsai://memory resource.
    memory_path: Arc<Option<std::path::PathBuf>>,
    /// Pair token for Bearer auth (optional — empty string disables auth).
    auth_token: String,
}

// ── Request handler ───────────────────────────────────────────────────────────

async fn handle_mcp(
    State(state): State<McpState>,
    headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    // Optional Bearer token check
    if !state.auth_token.is_empty() {
        let ok = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|t| t == state.auth_token)
            .unwrap_or(false);
        if !ok {
            let resp = JsonRpcResponse::err(req.id.unwrap_or(Value::Null), -32001, "Unauthorized");
            return (StatusCode::UNAUTHORIZED, Json(json!(resp)));
        }
    }

    if req.jsonrpc != "2.0" {
        let resp = JsonRpcResponse::err(
            Value::Null,
            -32600,
            "Invalid Request: jsonrpc must be '2.0'",
        );
        return (StatusCode::OK, Json(json!(resp)));
    }

    let id = req.id.unwrap_or(Value::Null);
    let params = req.params.unwrap_or(Value::Null);
    let result = dispatch(&state, &req.method, &params).await;

    let resp = match result {
        Ok(v) => JsonRpcResponse::ok(id, v),
        Err((code, msg)) => JsonRpcResponse::err(id, code, msg),
    };
    (StatusCode::OK, Json(json!(resp)))
}

async fn dispatch(state: &McpState, method: &str, params: &Value) -> Result<Value, (i32, String)> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": MCP_VERSION,
            "capabilities": server_capabilities(),
            "serverInfo": {
                "name":    SERVER_NAME,
                "version": SERVER_VERSION,
            }
        })),

        "ping" => Ok(json!({})),

        // ── tools ─────────────────────────────────────────────────────────────
        "tools/list" => {
            let reg = state.registry.read().await;
            let tools: Vec<Value> = reg.definitions(None, None);
            Ok(json!({ "tools": tools }))
        }

        "tools/call" => {
            let tool_name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or((-32602, "Missing 'name' param".to_string()))?;
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);

            let reg = state.registry.read().await;
            let tool = reg
                .get(tool_name)
                .ok_or((-32601, format!("Tool '{tool_name}' not found")))?;

            let ctx = default_tool_context();
            match tool.execute(&args, &ctx).await {
                Ok(crate::tool_core::ToolOutput::Complete(v)) => Ok(json!({
                    "content": [{ "type": "text", "text": v.to_string() }],
                    "isError": false,
                })),
                Ok(crate::tool_core::ToolOutput::Streaming(_)) => Ok(json!({
                    "content": [{ "type": "text", "text": "Streaming tool — use SSE endpoint." }],
                    "isError": false,
                })),
                Err(e) => Ok(json!({
                    "content": [{ "type": "text", "text": e.to_string() }],
                    "isError": true,
                })),
            }
        }

        // ── resources ─────────────────────────────────────────────────────────
        "resources/list" => {
            let mut resources = vec![
                json!({
                    "uri":         "bonsai://workspace",
                    "name":        "Workspace",
                    "description": "Current open workspace file tree (top 2 levels).",
                    "mimeType":    "text/plain"
                }),
                json!({
                    "uri":         "bonsai://memory",
                    "name":        "Agent Memory",
                    "description": "Persistent key-value facts stored by BonsAI agents.",
                    "mimeType":    "application/jsonl"
                }),
            ];
            Ok(json!({ "resources": resources }))
        }

        "resources/read" => {
            let uri = params
                .get("uri")
                .and_then(|v| v.as_str())
                .ok_or((-32602, "Missing 'uri' param".to_string()))?;

            match uri {
                "bonsai://workspace" => {
                    let root = state.workspace_root.as_ref().as_deref().unwrap_or(".");
                    let tree = workspace_tree(root, 2, 80);
                    Ok(json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": tree,
                        }]
                    }))
                }
                "bonsai://memory" => {
                    let content = state
                        .memory_path
                        .as_ref()
                        .as_ref()
                        .and_then(|p| std::fs::read_to_string(p).ok())
                        .unwrap_or_default();
                    Ok(json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/jsonl",
                            "text": content,
                        }]
                    }))
                }
                _ => Err((-32602, format!("Unknown resource URI: {uri}"))),
            }
        }

        // ── prompts ───────────────────────────────────────────────────────────
        "prompts/list" => Ok(json!({ "prompts": builtin_prompts() })),

        "prompts/get" => {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or((-32602, "Missing 'name' param".to_string()))?;
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            render_prompt(name, &args)
                .map(|p| p)
                .ok_or((-32602, format!("Unknown prompt: {name}")))
        }

        _ => Err((-32601, format!("Method not found: {method}"))),
    }
}

// ── SSE health ────────────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "server": SERVER_NAME,
        "version": SERVER_VERSION,
        "protocol": MCP_VERSION,
    }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn default_tool_context() -> crate::tool_core::ToolContext {
    use std::sync::{atomic::AtomicBool, Arc};
    crate::tool_core::ToolContext {
        workspace_path: None,
        profile_id: "mcp".into(),
        session_id: "mcp-session".into(),
        turn_id: uuid_short(),
        call_depth: 0,
        cancel: Arc::new(AtomicBool::new(false)),
        secrets: Arc::new(crate::secrets_store::SecretsStore::new()),
    }
}

fn uuid_short() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

fn workspace_tree(root: &str, max_depth: usize, max_entries: usize) -> String {
    let mut lines = Vec::new();
    tree_dir(
        std::path::Path::new(root),
        0,
        max_depth,
        &mut lines,
        max_entries,
    );
    lines.join("\n")
}

fn tree_dir(
    path: &std::path::Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<String>,
    limit: usize,
) {
    if depth > max_depth || out.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    let mut sorted: Vec<_> = entries.flatten().collect();
    sorted.sort_by_key(|e| e.file_name());
    for entry in sorted {
        if out.len() >= limit {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.')
            || matches!(
                name.as_str(),
                "node_modules" | "target" | "dist" | "__pycache__" | ".git" | ".svelte-kit"
            )
        {
            continue;
        }
        let indent = "  ".repeat(depth);
        let ep = entry.path();
        if ep.is_dir() {
            out.push(format!("{indent}{name}/"));
            tree_dir(&ep, depth + 1, max_depth, out, limit);
        } else {
            out.push(format!("{indent}{name}"));
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub struct McpServerHandle {
    pub port: u16,
}

/// Start the MCP server on an available port and return the handle.
/// The registry is shared — tool registrations made after startup are
/// immediately visible to MCP clients without restart.
pub async fn start(
    registry: Arc<RwLock<ToolRegistry>>,
    workspace_root: Option<String>,
    memory_path: Option<std::path::PathBuf>,
    auth_token: String,
    preferred_port: u16,
) -> Result<McpServerHandle, String> {
    let state = McpState {
        registry,
        workspace_root: Arc::new(workspace_root),
        memory_path: Arc::new(memory_path),
        auth_token,
    };

    let app = Router::new()
        .route("/mcp", post(handle_mcp))
        .route("/mcp/health", get(health))
        .with_state(state);

    // Try preferred port, fall back to OS-assigned
    let addr: SocketAddr = format!("127.0.0.1:{preferred_port}")
        .parse()
        .map_err(|e| format!("Invalid MCP address: {e}"))?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("[mcp] Failed to bind port {preferred_port}: {e}"))?;

    let port = listener
        .local_addr()
        .map(|a| a.port())
        .unwrap_or(preferred_port);
    info!(
        port,
        "[mcp] MCP server listening — connect via Claude Desktop, Cursor, or any MCP client"
    );

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            warn!(error=%e, "[mcp] server error");
        }
    });

    Ok(McpServerHandle { port })
}
