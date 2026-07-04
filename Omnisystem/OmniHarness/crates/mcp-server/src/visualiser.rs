use serde_json::{json, Value};
use tokio::sync::broadcast;
use std::sync::Arc;
use std::path::PathBuf;
use chrono::Utc;

/// The two agent control modes.
#[derive(Clone, PartialEq, Debug)]
pub enum VisualMode {
    Visual,
    Headless,
}

/// Configuration for headless mode flags.
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
            log_path: PathBuf::from("bonsai-agent.log"),
        }
    }
}

/// Shared state for the visualiser proxy.
pub struct VisualiserState {
    pub mode: VisualMode,
    pub headless_config: HeadlessConfig,
    pub event_tx: broadcast::Sender<VisualEvent>,
}

/// Events emitted for every agent action.
#[derive(Clone, serde::Serialize, Debug)]
#[serde(tag = "type")]
pub enum VisualEvent {
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
        reason: String,
        requires_approval: bool,
    },
    AgentResumed {
        timestamp: String,
        approved: bool,
    },
    SystemNotification {
        timestamp: String,
        level: String,
        message: String,
    },
}

impl VisualiserState {
    /// Create a new visualiser state.
    pub fn new(mode: VisualMode) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            mode,
            headless_config: HeadlessConfig::default(),
            event_tx,
        }
    }

    /// Create with a custom headless configuration.
    pub fn headless_with_config(config: HeadlessConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            mode: VisualMode::Headless,
            headless_config: config,
            event_tx,
        }
    }

    /// Wrap an MCP tool call, logging and optionally pausing for approval.
    pub async fn handle_mcp_call(
        &self,
        tool: &str,
        args: Value,
        token: &str,
    ) -> Result<Value, String> {
        let now = Utc::now().to_rfc3339();
        let start = std::time::Instant::now();

        // Check if this is a destructive operation that requires approval
        let requires_approval = is_destructive(tool);
        if requires_approval && self.headless_config.popup_on_approval {
            let _ = self.event_tx.send(VisualEvent::AgentPaused {
                timestamp: now.clone(),
                reason: format!("Destructive operation: {}", tool),
                requires_approval: true,
            });
            // In headless mode, write to log and return error if approval needed
            if self.mode == VisualMode::Headless {
                let msg = format!("Approval required for: {}", tool);
                self.log_to_file(&msg);
                self.send_notification("Approval Required", &msg);
                return Err("Approval required for destructive operation".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let _ = self.event_tx.send(VisualEvent::AgentResumed {
                timestamp: Utc::now().to_rfc3339(),
                approved: true,
            });
        }

        // Emit start event
        let start_event = VisualEvent::ToolCallStart {
            timestamp: now.clone(),
            tool: tool.into(),
            args: args.clone(),
        };
        let _ = self.event_tx.send(start_event);

        // Execute the actual tool call
        let result = crate::bridge::call_bonsai(token, tool, args).await;
        let duration = start.elapsed().as_millis() as u64;

        // Emit end event
        match &result {
            Ok(v) => {
                let end_event = VisualEvent::ToolCallEnd {
                    timestamp: Utc::now().to_rfc3339(),
                    tool: tool.into(),
                    result: Some(v.clone()),
                    error: None,
                    duration_ms: duration,
                };
                let _ = self.event_tx.send(end_event);

                // Headless notifications
                if self.mode == VisualMode::Headless {
                    if self.headless_config.verbose {
                        println!("✅ {} completed in {}ms", tool, duration);
                    }
                    if self.headless_config.notify_on_success && duration > 5000 {
                        self.send_notification(
                            "Task Complete",
                            &format!("{} completed in {}ms", tool, duration),
                        );
                    }
                }
            }
            Err(e) => {
                let end_event = VisualEvent::ToolCallEnd {
                    timestamp: Utc::now().to_rfc3339(),
                    tool: tool.into(),
                    result: None,
                    error: Some(e.to_string()),
                    duration_ms: duration,
                };
                let _ = self.event_tx.send(end_event);

                // Headless error notification
                if self.mode == VisualMode::Headless && self.headless_config.notify_on_error {
                    self.send_notification(
                        "Agent Error",
                        &format!("{} failed: {}", tool, e),
                    );
                }
            }
        }

        result.map_err(|e| e.to_string())
    }

    fn log_to_file(&self, msg: &str) {
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
                .app_name("Bonsai Agent")
                .show();
        }
        #[cfg(not(target_os = "windows"))]
        {
            tracing::info!("[{}] {}", title, body);
        }
    }
}

/// Determine if a tool call is destructive and requires user approval.
fn is_destructive(tool: &str) -> bool {
    matches!(
        tool,
        "write_file" | "delete_file" | "deploy_model" | "run_cargo_publish" | "git_force_push"
    )
}
