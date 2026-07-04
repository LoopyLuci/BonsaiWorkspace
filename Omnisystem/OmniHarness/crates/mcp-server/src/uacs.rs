use serde_json::{json, Value};
use tokio::sync::{broadcast, oneshot, Mutex};
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    response::IntoResponse,
    Router,
    routing::{get, post},
    Json,
    http::StatusCode,
};
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

// ── Mode & Config ──────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
pub enum UacsMode {
    Visual,
    Headless,
}

#[derive(Clone, Debug)]
pub struct HeadlessConfig {
    pub quiet: bool,
    pub verbose: bool,
    pub notify_on_error: bool,
    pub notify_on_success: bool,
    pub popup_on_approval: bool,
    pub log_path: PathBuf,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            quiet: false,
            verbose: true,
            notify_on_error: true,
            notify_on_success: false,
            popup_on_approval: true,
            log_path: PathBuf::from("uacs-agent.log"),
        }
    }
}

/// Human-In-The-Loop configuration
#[derive(Clone, Debug)]
pub struct HITLConfig {
    pub enabled: bool,
    pub approval_categories: Vec<ApprovalCategory>,
    pub fallback_terminal: bool,
}

impl Default for HITLConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            approval_categories: vec![
                ApprovalCategory::Destructive,
                ApprovalCategory::Network,
            ],
            fallback_terminal: true,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ApprovalCategory {
    Destructive,
    Network,
    ModelMutation,
    SystemModification,
    RemoteFileTransfer,
    RemoteClipboardAccess,
    RemoteTunnelCreation,
    All,
}

impl ApprovalCategory {
    pub fn description(&self) -> &'static str {
        match self {
            ApprovalCategory::Destructive => "Destructive operations (write, delete, deploy)",
            ApprovalCategory::Network => "Network access (web search, remote calls)",
            ApprovalCategory::ModelMutation => "Model training or deployment",
            ApprovalCategory::SystemModification => "System configuration changes",
            ApprovalCategory::RemoteFileTransfer => "Remote file transfers via mobile session",
            ApprovalCategory::RemoteClipboardAccess => "Clipboard access on remote desktop",
            ApprovalCategory::RemoteTunnelCreation => "Creating tunnels for remote connections",
            ApprovalCategory::All => "All tool calls",
        }
    }
}

// ── Events ──────────────────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize, Debug)]
#[serde(tag = "type")]
pub enum UacsEvent {
    ToolCallStart {
        timestamp: String,
        tool: String,
        args: Value,
    },
    ToolCallEnd {
        timestamp: String,
        tool: String,
        result: Option<Value>,
        error: Option<String>,
        duration_ms: u64,
    },
    ChatMessageSent {
        timestamp: String,
        role: String,
        content: String,
        model: Option<String>,
    },
    FileModified {
        timestamp: String,
        path: String,
        operation: String,
        size_bytes: Option<usize>,
    },
    TestRun {
        timestamp: String,
        command: String,
        success: bool,
        output: String,
        duration_ms: u64,
    },
    ModelPullProgress {
        timestamp: String,
        model: String,
        progress_percent: f32,
        status: String,
    },
    AgentPaused {
        timestamp: String,
        request_id: String,
        tool: String,
        description: String,
        risk: String,
        details: Value,
    },
    AgentResumed {
        timestamp: String,
        request_id: String,
        approved: bool,
    },
    SystemNotification {
        timestamp: String,
        level: String,
        message: String,
    },
    // Mobile Remote Desktop events
    RemoteSessionStarted {
        timestamp: String,
        session_id: String,
        peer_id: String,
        connection_type: String,
    },
    RemoteSessionEnded {
        timestamp: String,
        session_id: String,
        duration_secs: u64,
    },
    RemoteFileTransferRequest {
        timestamp: String,
        session_id: String,
        request_id: String,
        file_path: String,
        direction: String, // "upload" or "download"
        size_bytes: u64,
    },
    RemoteClipboardAccess {
        timestamp: String,
        session_id: String,
        request_id: String,
        operation: String, // "read" or "write"
    },
    RemoteTunnelCreated {
        timestamp: String,
        session_id: String,
        tunnel_id: String,
        latency_ms: f32,
    },
    RemoteSessionStats {
        timestamp: String,
        session_id: String,
        fps: f32,
        bitrate_mbps: f32,
        latency_ms: f32,
        bandwidth_usage_mb: f64,
    },
}

// ── Pending Approval ────────────────────────────────────────────────────────

struct PendingApproval {
    sender: oneshot::Sender<bool>,
    tool: String,
    description: String,
    risk: String,
    _args: Value,
}

// ── Shared State ────────────────────────────────────────────────────────────

pub struct UacsState {
    pub mode: UacsMode,
    pub headless_config: HeadlessConfig,
    pub hitl_config: HITLConfig,
    pub event_tx: broadcast::Sender<UacsEvent>,
    pending_approvals: Arc<Mutex<HashMap<String, PendingApproval>>>,
}

impl UacsState {
    pub fn new(mode: UacsMode) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            mode,
            headless_config: HeadlessConfig::default(),
            hitl_config: HITLConfig::default(),
            event_tx,
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn headless_with_config(config: HeadlessConfig, hitl: HITLConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            mode: UacsMode::Headless,
            headless_config: config,
            hitl_config: hitl,
            event_tx,
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn visual_with_hitl(hitl: HITLConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            mode: UacsMode::Visual,
            headless_config: HeadlessConfig::default(),
            hitl_config: hitl,
            event_tx,
            pending_approvals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // ── Tool Call Wrapper with HITL ────────────────────────────────────────

    pub async fn handle_tool_call(
        &self,
        tool: &str,
        args: Value,
        token: &str,
    ) -> Result<Value, String> {
        let now = Utc::now().to_rfc3339();
        let start_time = std::time::Instant::now();

        // Check if HITL approval is required
        let requires_approval = self.hitl_config.enabled && self.needs_approval(tool);
        if requires_approval {
            let request_id = Uuid::new_v4().to_string();
            let risk = self.assess_risk(tool);
            let description = self.describe_operation(tool, &args);

            let (tx, rx) = oneshot::channel();
            {
                let mut pending = self.pending_approvals.lock().await;
                pending.insert(request_id.clone(), PendingApproval {
                    sender: tx,
                    tool: tool.to_string(),
                    description: description.clone(),
                    risk: risk.clone(),
                    _args: args.clone(),
                });
            }

            // Emit the pause event
            let pause_event = UacsEvent::AgentPaused {
                timestamp: now.clone(),
                request_id: request_id.clone(),
                tool: tool.to_string(),
                description: description.clone(),
                risk: risk.clone(),
                details: args.clone(),
            };
            let _ = self.event_tx.send(pause_event);

            // Wait for user response
            let approved = match self.mode {
                UacsMode::Visual => {
                    match rx.await {
                        Ok(decision) => decision,
                        Err(_) => {
                            self.log(&format!("HITL channel closed without response for {} — auto-denying", tool));
                            false
                        }
                    }
                }
                UacsMode::Headless => {
                    self.request_headless_approval(&request_id, tool, &description, &risk, rx).await
                }
            };

            // Emit resume event
            let resume_event = UacsEvent::AgentResumed {
                timestamp: Utc::now().to_rfc3339(),
                request_id: request_id.clone(),
                approved,
            };
            let _ = self.event_tx.send(resume_event);

            // Clean up
            {
                let mut pending = self.pending_approvals.lock().await;
                pending.remove(&request_id);
            }

            if !approved {
                return Err(format!("User denied the operation: {}", tool));
            }
        }

        // Proceed with the actual tool call
        let start_event = UacsEvent::ToolCallStart {
            timestamp: now.clone(),
            tool: tool.into(),
            args: args.clone(),
        };
        let _ = self.event_tx.send(start_event);

        let result = crate::bridge::call_bonsai(token, tool, args).await;
        let duration = start_time.elapsed().as_millis() as u64;

        match &result {
            Ok(v) => {
                let end_event = UacsEvent::ToolCallEnd {
                    timestamp: Utc::now().to_rfc3339(),
                    tool: tool.into(),
                    result: Some(v.clone()),
                    error: None,
                    duration_ms: duration,
                };
                let _ = self.event_tx.send(end_event);

                if self.mode == UacsMode::Headless {
                    if self.headless_config.verbose {
                        println!("✅ {} completed in {}ms", tool, duration);
                    }
                    if self.headless_config.notify_on_success && duration > 5000 {
                        self.send_notification(
                            "UACS — Task Complete",
                            &format!("{} completed in {}ms", tool, duration),
                        );
                    }
                }
            }
            Err(e) => {
                let end_event = UacsEvent::ToolCallEnd {
                    timestamp: Utc::now().to_rfc3339(),
                    tool: tool.into(),
                    result: None,
                    error: Some(e.to_string()),
                    duration_ms: duration,
                };
                let _ = self.event_tx.send(end_event);

                if self.mode == UacsMode::Headless && self.headless_config.notify_on_error {
                    self.send_notification(
                        "UACS — Agent Error",
                        &format!("{} failed: {}", tool, e),
                    );
                }
            }
        }

        result.map_err(|e| e.to_string())
    }

    // ── HITL Helpers ──────────────────────────────────────────────────────

    fn needs_approval(&self, tool: &str) -> bool {
        for cat in &self.hitl_config.approval_categories {
            match cat {
                ApprovalCategory::All => return true,
                ApprovalCategory::Destructive if is_destructive(tool) => return true,
                ApprovalCategory::Network if is_network_tool(tool) => return true,
                ApprovalCategory::ModelMutation if is_model_tool(tool) => return true,
                ApprovalCategory::SystemModification if is_system_tool(tool) => return true,
                _ => {}
            }
        }
        false
    }

    fn assess_risk(&self, tool: &str) -> String {
        if is_destructive(tool) {
            "high".into()
        } else if is_network_tool(tool) {
            "medium".into()
        } else {
            "low".into()
        }
    }

    fn describe_operation(&self, tool: &str, args: &Value) -> String {
        match tool {
            "write_file" => format!("Write file: {}", args["path"].as_str().unwrap_or("unknown")),
            "delete_file" => format!("Delete file: {}", args["path"].as_str().unwrap_or("unknown")),
            "deploy_model" => format!("Deploy model: {}", args["adapter_name"].as_str().unwrap_or("unknown")),
            "web_search" => format!("Search web for: {}", args["query"].as_str().unwrap_or("unknown")),
            "run_cargo_publish" => "Publish crate to registry".into(),
            "git_force_push" => format!("Force push to: {}", args["branch"].as_str().unwrap_or("unknown")),
            _ => format!("Execute tool: {}", tool),
        }
    }

    async fn request_headless_approval(
        &self,
        _request_id: &str,
        tool: &str,
        description: &str,
        risk: &str,
        rx: oneshot::Receiver<bool>,
    ) -> bool {
        // Try interactive notification first
        if let Ok(_notification) = notify_rust::Notification::new()
            .summary(&format!("🔔 UACS Approval — {}", tool))
            .body(&format!("{}\n\nRisk: {}", description, risk))
            .icon("dialog-question")
            .show()
        {
            // Wait for user response with 30-second timeout
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                    self.log(&format!("HITL approval timeout for {} — auto-denying", tool));
                    self.send_notification("UACS — Auto-Denied", &format!("Approval timeout for {}", tool));
                    false
                }
                result = rx => result.unwrap_or(false),
            }
        } else if self.hitl_config.fallback_terminal {
            // Fallback: terminal prompt
            println!("\n════════════════════════════════════════════════════════");
            println!("🔔 UACS Approval Required");
            println!("════════════════════════════════════════════════════════");
            println!("Tool:        {}", tool);
            println!("Description: {}", description);
            println!("Risk Level:  {} 🔴", risk.to_uppercase());
            println!("────────────────────────────────────────────────────────");
            println!("Approve this operation? (y/N): ");
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_ok() {
                let approved = input.trim().to_lowercase() == "y";
                println!("Response: {}", if approved { "✅ APPROVED" } else { "❌ DENIED" });
                approved
            } else {
                false
            }
        } else {
            // Auto-deny
            self.log(&format!("HITL not interactive — auto-denying {}", tool));
            false
        }
    }

    fn log(&self, msg: &str) {
        let entry = format!("[{}] {}\n", Utc::now().to_rfc3339(), msg);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.headless_config.log_path)
        {
            use std::io::Write;
            let _ = file.write_all(entry.as_bytes());
        }
    }

    fn send_notification(&self, title: &str, body: &str) {
        #[cfg(target_os = "windows")]
        {
            let _ = notify_rust::Notification::new()
                .summary(title)
                .body(body)
                .appname("Universal Agent Control System")
                .show();
        }
        #[cfg(not(target_os = "windows"))]
        {
            tracing::info!("[{}] {}", title, body);
        }
    }

    // ── WebSocket ─────────────────────────────────────────────────────────

    pub async fn ws_handler(ws: WebSocketUpgrade, state: Arc<Self>) -> impl IntoResponse {
        ws.on_upgrade(|socket| Self::handle_websocket(socket, state))
    }

    async fn handle_websocket(mut socket: WebSocket, state: Arc<Self>) {
        let mut rx = state.event_tx.subscribe();
        while let Ok(event) = rx.recv().await {
            let msg = match serde_json::to_string(&event) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if socket.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    }

    // ── Dashboard Response Handler ─────────────────────────────────────────

    pub async fn respond_to_approval(
        pending: &Arc<Mutex<HashMap<String, PendingApproval>>>,
        request_id: &str,
        approved: bool,
    ) -> bool {
        let mut map = pending.lock().await;
        if let Some(pending) = map.remove(request_id) {
            let _ = pending.sender.send(approved);
            true
        } else {
            false
        }
    }
}

// ── Tool Categorisation ─────────────────────────────────────────────────────

fn is_destructive(tool: &str) -> bool {
    matches!(
        tool,
        "write_file" | "delete_file" | "run_cargo_publish" | "git_force_push" | "deploy_model"
    )
}

fn is_network_tool(tool: &str) -> bool {
    matches!(
        tool,
        "web_search" | "create_collaboration_session" | "get_peers" | "http_request"
    )
}

fn is_model_tool(tool: &str) -> bool {
    matches!(tool, "train" | "deploy_model" | "create_model" | "fine_tune")
}

fn is_system_tool(tool: &str) -> bool {
    matches!(
        tool,
        "network_set_firewall_profile" | "network_toggle_adapter" | "run_cargo_publish"
    )
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn create_uacs_router(state: Arc<UacsState>) -> Router {
    let pending = state.pending_approvals.clone();
    let state_mcp = state.clone();
    let state_ws = state.clone();
    Router::new()
        .route(
            "/mcp",
            post(move |headers, body| handle_mcp_request(state_mcp.clone(), headers, body)),
        )
        .route(
            "/ws/events",
            get(move |ws| UacsState::ws_handler(ws, state_ws.clone())),
        )
        .route(
            "/api/respond",
            post(move |Json(body): Json<Value>| async move {
                let request_id = body["request_id"].as_str().unwrap_or("");
                let approved = body["approved"].as_bool().unwrap_or(false);
                let success = UacsState::respond_to_approval(&pending, request_id, approved).await;
                if success {
                    (StatusCode::OK, Json(json!({"status": "ok"})))
                } else {
                    (StatusCode::NOT_FOUND, Json(json!({"status": "not_found"})))
                }
            }),
        )
        .route("/health", get(|| async { "OK" }))
}

async fn handle_mcp_request(
    state: Arc<UacsState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let method = body["method"].as_str().unwrap_or("");
    let params = body.get("params").cloned().unwrap_or(json!({}));
    let id = body.get("id").cloned().unwrap_or(json!(null));

    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("default-token");

    match method {
        "tools/list" => {
            let tools = crate::tools::list_tools();
            Ok(Json(json!({"jsonrpc": "2.0", "id": id, "result": {"tools": tools}})))
        }
        "tools/call" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let tool_args = params.get("arguments").cloned().unwrap_or(json!({}));
            match state.handle_tool_call(tool_name, tool_args, token).await {
                Ok(result) => Ok(Json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"content": [{"type": "text", "text": result.to_string()}]}
                }))),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"code": -32000, "message": e}
                    })),
                )),
            }
        }
        "initialize" => Ok(Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "Universal Agent Control System", "version": "1.0.0"}
            }
        }))),
        _ => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": "Method not found"}
            })),
        )),
    }
}
