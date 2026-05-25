//! Pluggable tool registry — tools register themselves at startup and can be
//! invoked by the assistant pipeline or directly via Tauri commands.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::warn;

// ── Tool trait ────────────────────────────────────────────────────────────────

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn run(&self, args: &Value) -> Result<ToolResult, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// MIME type of the result (e.g. "text/plain", "image/png", "audio/wav", "application/json").
    pub content_type: String,
    /// Raw bytes of the result.
    pub data: Vec<u8>,
}

impl ToolResult {
    pub fn text(s: impl Into<String>) -> Self {
        Self { content_type: "text/plain".into(), data: s.into().into_bytes() }
    }
    pub fn json(v: &Value) -> Self {
        Self {
            content_type: "application/json".into(),
            data: serde_json::to_vec(v).unwrap_or_default(),
        }
    }
    pub fn as_text(&self) -> Option<&str> {
        if self.content_type.starts_with("text/") {
            std::str::from_utf8(&self.data).ok()
        } else {
            None
        }
    }
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Box<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: RwLock::new(HashMap::new()) }
    }

    pub async fn register(&self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.write().await.insert(name, tool);
    }

    pub async fn execute(&self, name: &str, args: &Value) -> Option<ToolResult> {
        let tools = self.tools.read().await;
        match tools.get(name)?.run(args).await {
            Ok(r) => Some(r),
            Err(e) => {
                warn!(tool=name, error=%e, "[tool_registry] execution failed");
                None
            }
        }
    }

    pub async fn list(&self) -> Vec<ToolInfo> {
        self.tools
            .read()
            .await
            .values()
            .map(|t| ToolInfo { name: t.name().to_string(), description: t.description().to_string() })
            .collect()
    }
}

#[derive(Debug, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

// ── Built-in: ExecuteCodeTool ─────────────────────────────────────────────────

pub struct ExecuteCodeTool;

#[async_trait]
impl Tool for ExecuteCodeTool {
    fn name(&self) -> &str { "execute_code" }
    fn description(&self) -> &str { "Execute Python code in a sandboxed venv. Args: {code: string, timeout_secs?: number}" }

    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code = args["code"].as_str().ok_or("Missing 'code' argument")?;
        let timeout_secs = args["timeout_secs"].as_u64();
        let result = crate::sandbox_executor::execute_plugin_code(code).await?;
        let output = serde_json::json!({
            "stdout": result.stdout,
            "stderr": result.stderr,
            "exit_code": result.exit_code,
            "timed_out": result.timed_out,
        });
        Ok(ToolResult::json(&output))
    }
}

// ── Built-in: SystemInfoTool ──────────────────────────────────────────────────

pub struct SystemInfoTool;

#[async_trait]
impl Tool for SystemInfoTool {
    fn name(&self) -> &str { "system_info" }
    fn description(&self) -> &str { "Return CPU, RAM, and OS info." }

    async fn run(&self, _args: &Value) -> Result<ToolResult, String> {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();
        let info = serde_json::json!({
            "os": System::name(),
            "kernel": System::kernel_version(),
            "cpu_count": sys.cpus().len(),
            "total_ram_mb": sys.total_memory() / 1024 / 1024,
            "used_ram_mb": sys.used_memory() / 1024 / 1024,
        });
        Ok(ToolResult::json(&info))
    }
}

// ── Shared state wrapper ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ToolRegistryState {
    pub registry: Arc<ToolRegistry>,
}

impl ToolRegistryState {
    pub async fn new_with_defaults() -> Arc<Self> {
        let registry = Arc::new(ToolRegistry::new());
        registry.register(Box::new(ExecuteCodeTool)).await;
        registry.register(Box::new(SystemInfoTool)).await;
        Arc::new(Self { registry })
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_tools(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<ToolInfo>, String> {
    Ok(state.tool_registry.registry.list().await)
}

#[tauri::command]
pub async fn run_tool(
    state: tauri::State<'_, crate::AppState>,
    name: String,
    args: Value,
) -> Result<ToolResult, String> {
    state
        .tool_registry
        .registry
        .execute(&name, &args)
        .await
        .ok_or_else(|| format!("Tool '{name}' not found or failed"))
}
