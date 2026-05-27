//! Pluggable tool registry — tools register themselves at startup and can be
//! invoked by the assistant pipeline or directly via Tauri commands.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::warn;
use bonsai_capability_registry::{CapabilityEntry, CapabilitySource, EffectRow};

// ── Tool trait ────────────────────────────────────────────────────────────────

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn run(&self, args: &Value) -> Result<ToolResult, String>;

    /// Optional streaming run with progress updates sent on `progress_tx`.
    /// By default, this calls `run()` and sends no progress updates.
    async fn run_with_progress(
        &self,
        args: &Value,
        _progress_tx: tokio::sync::mpsc::UnboundedSender<serde_json::Value>,
    ) -> Result<ToolResult, String> {
        self.run(args).await
    }
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

    /// Execute a tool and forward progress via `progress_tx`.
    pub async fn execute_with_progress(
        &self,
        name: &str,
        args: &Value,
        progress_tx: tokio::sync::mpsc::UnboundedSender<serde_json::Value>,
    ) -> Option<ToolResult> {
        let tools = self.tools.read().await;
        let tool = tools.get(name)?;
        match tool.run_with_progress(args, progress_tx).await {
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

// Expose the tool registry state as a CapabilitySource for the Universal Capability Registry.
impl CapabilitySource for ToolRegistryState {
    fn source_id(&self) -> &str { "tool_registry" }
    fn source_type(&self) -> &str { "tool_registry" }

    fn generate_entries(&self) -> Vec<CapabilityEntry> {
        let infos = self.list_tools();
        let mut out = Vec::new();
        for info in infos {
            // Build trigger phrases from tool name + meaningful words in description
            let mut triggers = vec![info.name.clone()];
            let desc_words: Vec<String> = info.description.split_whitespace()
                .filter(|w| w.len() > 3)
                .take(8)
                .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| !w.is_empty())
                .collect();
            if !desc_words.is_empty() {
                triggers.push(desc_words.join(" "));
            }
            // Extra triggers for well-known tools
            match info.name.as_str() {
                "system_info" | "get_system_stats" => {
                    triggers.extend(["specs".into(), "system specs".into(), "hardware info".into(),
                        "cpu info".into(), "ram info".into(), "what are my specs".into()]);
                }
                "execute_code" => {
                    triggers.extend(["run code".into(), "execute python".into()]);
                }
                _ => {}
            }
            let entry = CapabilityEntry {
                name: info.name.clone(),
                id: info.name.clone(),
                category: "tools".to_string(),
                description: Some(info.description.clone()),
                trigger_phrases: triggers,
                capability_tags: vec!["tool".to_string(), "registry".to_string()],
                parameters: serde_json::Value::Null,
                examples: vec![],
                requires_model: None,
                effect_row: EffectRow::default(),
                trust_level: "L2".to_string(),
                availability: None,
                version: None,
                content_hash: None,
            };
            out.push(entry);
        }
        out
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
        // Demo streaming tool for testing progress updates
        registry.register(Box::new(crate::tools::demo_streaming::DemoStreamingTool::new())).await;
        let state = Arc::new(Self { registry });
        // Register Phase-1 multi-modal tools (Kokoro TTS, Depth, YOLO).
        crate::multimodal::register_all(&state).await;
        // Register expanded tool library (60+ tools across all categories).
        for tool in crate::expanded_tools::all_expanded_tools() {
            state.registry.register(Box::new(ExpandedToolWrapper(tool))).await;
        }
        // Register tools submodules.
        for tool in crate::tools::ai_code_tools::all_ai_code_tools() {
            state.registry.register(Box::new(ExpandedToolWrapper(tool))).await;
        }
        for tool in crate::tools::data_science_tools::all_data_science_tools() {
            state.registry.register(Box::new(ExpandedToolWrapper(tool))).await;
        }
        for tool in crate::tools::security_tools::all_security_tools() {
            state.registry.register(Box::new(ExpandedToolWrapper(tool))).await;
        }
        for tool in crate::tools::creative_ext_tools::all_creative_ext_tools() {
            state.registry.register(Box::new(ExpandedToolWrapper(tool))).await;
        }
        for tool in crate::tools::web_ext_tools::all_web_ext_tools() {
            state.registry.register(Box::new(ExpandedToolWrapper(tool))).await;
        }
        state
    }

    /// Return a simple list of tools for external listing consumers.
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        // Note: This clones the current snapshot; callers should be quick.
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async { self.registry.list().await })
    }

    /// Convert all registered tools to `ToolDef` for the ReAct prompt and tool resolution.
    /// Derived metadata: `requires_approval = false` (registry tools are sandboxed or safe),
    /// trigger phrases built from name + lowercase description words.
    pub fn to_tool_defs(&self) -> Vec<crate::tools::ToolDef> {
        use bonsai_capability_registry::EffectRow;
        self.list_tools().into_iter().map(|info| {
            // Derive trigger phrases: tool name + first few lowercase words of description
            let mut triggers = vec![info.name.clone()];
            let desc_words: Vec<String> = info.description.split_whitespace()
                .take(6)
                .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| w.len() > 2)
                .collect();
            if !desc_words.is_empty() {
                triggers.push(desc_words.join(" "));
            }
            crate::tools::ToolDef {
                name:              info.name,
                description:       info.description,
                args:              vec![],
                requires_approval: false,
                is_custom:         false,
                script_path:       None,
                trigger_phrases:   triggers,
                capability_tags:   vec!["tool".into(), "registry".into()],
                examples:          vec![],
                requires_model:    None,
                effect_row:        EffectRow::default(),
            }
        }).collect()
    }

    /// Check whether a tool name is present in this registry (sync, non-blocking).
    pub fn contains_tool(&self, name: &str) -> bool {
        self.list_tools().iter().any(|t| t.name == name)
    }

    /// Invoke a tool by name with a 30-second timeout.
    pub async fn invoke_by_name(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
        let fut = self.registry.execute(name, &args);
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), fut)
            .await
            .map_err(|_| format!("tool '{}' timed out after 30s", name))?;
        match result {
            Some(res) => {
                if res.content_type.starts_with("text/") {
                    if let Ok(s) = std::str::from_utf8(&res.data) {
                        Ok(serde_json::json!({ "content": s }))
                    } else {
                        Err("invalid utf8 in text result".into())
                    }
                } else if res.content_type == "application/json" {
                    serde_json::from_slice(&res.data).map_err(|e| e.to_string())
                } else {
                    Err(format!("unsupported content_type: {}", res.content_type))
                }
            }
            None => Err(format!("tool '{}' not found or failed", name)),
        }
    }
}

// ── Arc<dyn Tool> → Box<dyn Tool> adapter ────────────────────────────────────

struct ExpandedToolWrapper(Arc<dyn Tool>);

#[async_trait]
impl Tool for ExpandedToolWrapper {
    fn name(&self) -> &str { self.0.name() }
    fn description(&self) -> &str { self.0.description() }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> { self.0.run(args).await }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_tools(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<ToolInfo>, String> {
    Ok(state.tool_registry.registry.list().await)
}

#[tauri::command]
pub async fn discover_peers_cmd(
    _state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<(String,u16,Vec<String>)>, String> {
    match crate::p2p::sharing::discover_peers().await {
        Ok(p) => Ok(p),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn request_model_cmd(
    _state: tauri::State<'_, crate::AppState>,
    url: String,
    local_path: String,
) -> Result<(), String> {
    crate::p2p::sharing::request_model(&url, &local_path).await.map_err(|e| e.to_string())
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
