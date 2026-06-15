//! Tool Call Middleware Pipeline.
//!
//! Every tool call emitted by the assistant passes through a chain of
//! `ToolMiddleware` implementations before being dispatched.  Middlewares can:
//!   - Allow the call through unchanged.
//!   - Rewrite the call (change tool name or arguments).
//!   - Block the call and return an error to the caller.
//!   - Hold the call pending human approval (via `PlanGate`).
//!
//! The registry is stored in `AppState` and is consulted in `execute_tool_call`.
//!
//! Built-in middlewares (registered at startup in `lib.rs`):
//!   - `SafetyGateMiddleware`   — blocks shell commands that could destroy data
//!   - `UndercoverMiddleware`   — sanitizes product names when undercover mode is on
//!   - `PlanGateMiddleware`     — holds high-risk ops for user approval
//!
//! Adding a custom middleware at runtime:
//! ```rust
//! state.middleware_registry.register(Box::new(MyMiddleware));
//! ```

use std::sync::{Arc, RwLock};

use serde_json::Value;

// ── Core trait ────────────────────────────────────────────────────────────────

/// A tool call as seen by the middleware pipeline.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub args: Value,
    /// Caller-provided workspace context (may be None for headless calls).
    pub workspace_path: Option<String>,
}

impl ToolCall {
    pub fn new(tool: impl Into<String>, args: Value) -> Self {
        Self {
            tool: tool.into(),
            args,
            workspace_path: None,
        }
    }
}

/// Result of a middleware interception.
pub enum MiddlewareOutcome {
    /// Pass the (possibly rewritten) call to the next middleware.
    Continue(ToolCall),
    /// Block execution immediately, returning this error message to the caller.
    Block(String),
    /// Hold the call pending async human approval; the middleware is responsible
    /// for resolving it via `PlanGate`.  The outer dispatch loop will await the gate.
    PendingApproval,
}

pub trait ToolMiddleware: Send + Sync {
    fn name(&self) -> &'static str;
    fn intercept(&self, call: ToolCall) -> MiddlewareOutcome;
}

// ── Registry ──────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct MiddlewareRegistry {
    middlewares: RwLock<Vec<Arc<dyn ToolMiddleware>>>,
}

impl MiddlewareRegistry {
    pub fn new() -> Self {
        Self {
            middlewares: RwLock::new(Vec::new()),
        }
    }

    pub fn register(&self, mw: Arc<dyn ToolMiddleware>) {
        self.middlewares.write().unwrap().push(mw);
    }

    pub fn unregister(&self, name: &str) {
        self.middlewares
            .write()
            .unwrap()
            .retain(|m| m.name() != name);
    }

    /// Run the call through the full chain.  Returns `Ok(call)` if all
    /// middlewares approved, `Err(msg)` if any blocked.
    pub fn run(&self, call: ToolCall) -> Result<ToolCall, String> {
        let chain = self.middlewares.read().unwrap();
        let mut current = call;
        for mw in chain.iter() {
            match mw.intercept(current) {
                MiddlewareOutcome::Continue(next) => current = next,
                MiddlewareOutcome::Block(msg) => return Err(msg),
                MiddlewareOutcome::PendingApproval => {
                    return Err("__pending_approval__".to_string());
                }
            }
        }
        Ok(current)
    }
}

// ── Built-in: SafetyGateMiddleware ────────────────────────────────────────────

/// Blocks shell commands that could irreversibly destroy data or compromise
/// system security.  The blocklist covers the most common destructive patterns;
/// it is not exhaustive.
pub struct SafetyGateMiddleware;

const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "rm -rf ~",
    "rm -rf $HOME",
    "sudo rm",
    "chmod 777 /",
    "chmod -R 777 /",
    "> /dev/sda",
    "dd if=/dev/zero of=/dev/sd",
    "mkfs.",
    "format c:",
    "del /f /s /q c:\\",
    "rd /s /q c:\\windows",
    ":(){ :|:& };:", // fork bomb
];

impl ToolMiddleware for SafetyGateMiddleware {
    fn name(&self) -> &'static str {
        "safety_gate"
    }

    fn intercept(&self, call: ToolCall) -> MiddlewareOutcome {
        // Only inspect shell/terminal tools
        if !matches!(
            call.tool.as_str(),
            "run_command" | "run_terminal_command" | "omni_shell_exec" | "execute_code"
        ) {
            return MiddlewareOutcome::Continue(call);
        }

        let cmd = call.args["command"]
            .as_str()
            .or_else(|| call.args["code"].as_str())
            .unwrap_or("")
            .to_lowercase();

        for pattern in BLOCKED_PATTERNS {
            if cmd.contains(*pattern) {
                return MiddlewareOutcome::Block(format!(
                    "SafetyGate blocked a potentially destructive command: `{}`. \
                     This pattern ({}) is on the safety blocklist.",
                    call.args["command"].as_str().unwrap_or("(unknown)"),
                    pattern,
                ));
            }
        }
        MiddlewareOutcome::Continue(call)
    }
}

// ── Built-in: UndercoverMiddleware ────────────────────────────────────────────

/// When undercover mode is enabled, strips internal product names from
/// tool arguments (commit messages, file contents written to disk, etc.)
/// so they don't appear in project history.
pub struct UndercoverMiddleware {
    pub enabled: Arc<std::sync::atomic::AtomicBool>,
}

const INTERNAL_NAMES: &[(&str, &str)] = &[
    ("BonsAI", "AI Assistant"),
    ("Bonsai Workspace", "Development Environment"),
    ("bonsai-latest", "model-latest"),
    ("bonsai_core", "assistant_core"),
];

impl ToolMiddleware for UndercoverMiddleware {
    fn name(&self) -> &'static str {
        "undercover"
    }

    fn intercept(&self, mut call: ToolCall) -> MiddlewareOutcome {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return MiddlewareOutcome::Continue(call);
        }
        // Sanitize string values in the args JSON.
        sanitize_json_strings(&mut call.args);
        MiddlewareOutcome::Continue(call)
    }
}

fn sanitize_json_strings(val: &mut Value) {
    match val {
        Value::String(s) => {
            for (from, to) in INTERNAL_NAMES {
                *s = s.replace(from, to);
            }
        }
        Value::Object(map) => {
            for v in map.values_mut() {
                sanitize_json_strings(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                sanitize_json_strings(v);
            }
        }
        _ => {}
    }
}

// ── Built-in: PlanGateMiddleware ──────────────────────────────────────────────
// The plan gate is implemented in `plan_gate.rs` and registered from `lib.rs`.
// This stub just exports the middleware name constant for registry lookups.
pub const PLAN_GATE_MW_NAME: &str = "plan_gate";
