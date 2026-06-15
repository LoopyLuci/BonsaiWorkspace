//! Plan Review Gate — pauses high-risk tool calls for human approval.
//!
//! When the `PlanGateMiddleware` intercepts a risky operation it:
//!   1. Creates a `PendingPlan` entry with a oneshot channel.
//!   2. Emits a Tauri `plan-pending` event to the frontend.
//!   3. Suspends the tool dispatch goroutine until the user approves/rejects.
//!   4. The frontend calls `resolve_plan` which fires the channel.
//!
//! Approval is per-step: the user can approve some steps and reject others.
//! Rejected steps are returned as tool errors; the assistant can then decide
//! how to proceed.
//!
//! Risk heuristics (configurable via `config/training.yaml` in the future):
//!   - Writes to more than 3 files in one call
//!   - Shell commands matching medium-risk patterns (`sudo`, `git push --force`, etc.)
//!   - Deletes that are not in a temp or build directory

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

use crate::middleware::{MiddlewareOutcome, ToolCall, ToolMiddleware};

// ── Risk assessment ───────────────────────────────────────────────────────────

/// Risk tiers.  `High` triggers the plan gate; `Medium` is logged but allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Patterns that elevate a shell command to high risk.
const HIGH_RISK_PATTERNS: &[&str] = &[
    "git push --force",
    "git push -f ",
    "git reset --hard",
    "git clean -fd",
    "drop table",
    "truncate table",
    "delete from",
    "sudo ",
    "npm publish",
    "cargo publish",
];

pub fn assess_risk(call: &ToolCall) -> RiskLevel {
    let tool = call.tool.as_str();

    // Multi-file writes are medium risk
    if tool == "write_file" || tool == "omnfs_write" {
        return RiskLevel::Medium;
    }

    // File deletion is high risk (outside obvious temp dirs)
    if tool == "delete_file" || tool == "omnfs_delete" {
        let path = call.args["path"].as_str().unwrap_or("");
        let is_temp = path.contains("target/")
            || path.contains("tmp/")
            || path.contains("temp/")
            || path.contains(".cache/");
        if !is_temp {
            return RiskLevel::High;
        }
    }

    // Shell commands with high-risk patterns
    if matches!(
        tool,
        "run_command" | "run_terminal_command" | "omni_shell_exec"
    ) {
        let cmd = call.args["command"].as_str().unwrap_or("").to_lowercase();
        for pat in HIGH_RISK_PATTERNS {
            if cmd.contains(*pat) {
                return RiskLevel::High;
            }
        }
    }

    RiskLevel::Low
}

// ── Pending plan store ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PendingPlan {
    pub plan_id: String,
    pub tool: String,
    pub args: Value,
    pub risk: RiskLevel,
    pub workspace: Option<String>,
}

type ResolveSender = oneshot::Sender<bool>;

pub struct PlanGateState {
    pending: Mutex<HashMap<String, ResolveSender>>,
}

impl PlanGateState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Register a pending plan and return the rx to await.
    pub fn register(&self, plan_id: &str, tx: ResolveSender) {
        self.pending.lock().unwrap().insert(plan_id.to_string(), tx);
    }

    /// Resolve a pending plan.  Returns false if plan_id not found.
    pub fn resolve(&self, plan_id: &str, approved: bool) -> bool {
        if let Some(tx) = self.pending.lock().unwrap().remove(plan_id) {
            let _ = tx.send(approved);
            true
        } else {
            false
        }
    }
}

// ── Middleware implementation ─────────────────────────────────────────────────

pub struct PlanGateMiddleware {
    pub state: Arc<PlanGateState>,
    pub app_handle: AppHandle,
    pub enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl ToolMiddleware for PlanGateMiddleware {
    fn name(&self) -> &'static str {
        crate::middleware::PLAN_GATE_MW_NAME
    }

    fn intercept(&self, call: ToolCall) -> MiddlewareOutcome {
        if !self.enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return MiddlewareOutcome::Continue(call);
        }

        if assess_risk(&call) != RiskLevel::High {
            return MiddlewareOutcome::Continue(call);
        }

        let plan_id = uuid::Uuid::new_v4().to_string();
        let plan = PendingPlan {
            plan_id: plan_id.clone(),
            tool: call.tool.clone(),
            args: call.args.clone(),
            risk: RiskLevel::High,
            workspace: call.workspace_path.clone(),
        };

        let _ = self.app_handle.emit("plan-pending", &plan);

        // Register the oneshot channel.
        let (tx, rx) = oneshot::channel::<bool>();
        self.state.register(&plan_id, tx);

        // Block the current thread waiting for approval.
        // The frontend must call `resolve_plan` within 5 minutes.
        let approved = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(300), rx)
                    .await
                    .unwrap_or(Ok(false))
                    .unwrap_or(false)
            })
        });

        if approved {
            MiddlewareOutcome::Continue(call)
        } else {
            MiddlewareOutcome::Block(format!(
                "High-risk operation '{}' was rejected by the Plan Review Gate.",
                call.tool
            ))
        }
    }
}

// ── Tauri command helpers ─────────────────────────────────────────────────────

/// Called from `commands.rs` — resolves a pending plan.
pub fn resolve(state: &PlanGateState, plan_id: &str, approved: bool) -> Result<(), String> {
    if state.resolve(plan_id, approved) {
        Ok(())
    } else {
        Err(format!("No pending plan with id '{plan_id}'"))
    }
}
