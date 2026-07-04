// ── Substrate governance (Rust kernel) ───────────────────────────────────────
// The trust anchor for autonomous runs: enforces resource budgets, capability
// policy, a SHA-256 hash-chained audit log, and a live kill switch — in the
// kernel, so no orchestrator bug can let a swarm/evolution run exceed its bounds.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_model_calls: u32,
    pub max_tokens: u64,
    pub max_cost_usd: f64,
    pub max_steps: u32,
    pub max_wallclock_ms: u64,
    pub max_parallel: u32,
}

impl Default for Budget {
    fn default() -> Self {
        Budget {
            max_model_calls: 200,
            max_tokens: 2_000_000,
            max_cost_usd: 10.0,
            max_steps: 500,
            max_wallclock_ms: 1_800_000,
            max_parallel: 16,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub model_calls: u32,
    pub tokens: u64,
    pub cost_usd: f64,
    pub steps: u32,
    pub started_ms: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityPolicy {
    pub allowed_models: HashSet<String>, // empty = any
    pub allowed_tools: HashSet<String>,  // empty = any
    pub denied_tools: HashSet<String>,
    pub allow_network: bool,
    pub allow_fs_write: bool,
    pub max_agents: u32,
}

#[derive(Debug)]
pub enum GovError {
    BudgetExceeded(String),
    PolicyViolation(String),
    Aborted(String),
}

impl std::fmt::Display for GovError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GovError::BudgetExceeded(s) => write!(f, "budget exceeded: {s}"),
            GovError::PolicyViolation(s) => write!(f, "policy violation: {s}"),
            GovError::Aborted(s) => write!(f, "aborted: {s}"),
        }
    }
}

// ── Hash-chained audit log ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub seq: u64,
    pub timestamp_ms: i64,
    pub kind: String,
    pub payload: String,
    pub prev_hash: String,
    pub hash: String,
}

#[derive(Default)]
pub struct AuditChain {
    events: Vec<AuditEvent>,
    head: String,
}

impl AuditChain {
    pub fn new() -> Self {
        AuditChain { events: Vec::new(), head: "0".repeat(64) }
    }

    fn hash(head: &str, ts: i64, kind: &str, payload: &str) -> String {
        let mut h = Sha256::new();
        h.update(head.as_bytes());
        h.update(ts.to_string().as_bytes());
        h.update(kind.as_bytes());
        h.update(payload.as_bytes());
        format!("{:x}", h.finalize())
    }

    pub fn append(&mut self, kind: &str, payload: &str) -> String {
        let ts = chrono::Utc::now().timestamp_millis();
        let digest = Self::hash(&self.head, ts, kind, payload);
        self.events.push(AuditEvent {
            seq: self.events.len() as u64,
            timestamp_ms: ts,
            kind: kind.to_string(),
            payload: payload.to_string(),
            prev_hash: self.head.clone(),
            hash: digest.clone(),
        });
        self.head = digest.clone();
        digest
    }

    pub fn verify(&self) -> bool {
        let mut head = "0".repeat(64);
        for ev in &self.events {
            let digest = Self::hash(&head, ev.timestamp_ms, &ev.kind, &ev.payload);
            if digest != ev.hash || ev.prev_hash != head {
                return false;
            }
            head = digest;
        }
        true
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }
}

// ── Kill switch ───────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct KillSwitch {
    tripped: bool,
    reason: String,
}

impl KillSwitch {
    pub fn trip(&mut self, reason: &str) {
        self.tripped = true;
        self.reason = reason.to_string();
    }
    pub fn tripped(&self) -> bool {
        self.tripped
    }
}

// ── Governor: enforces everything, shareable across async tasks ───────────────

pub struct Governor {
    pub budget: Budget,
    pub policy: CapabilityPolicy,
    pub usage: Usage,
    pub kill: KillSwitch,
    pub audit: AuditChain,
}

const COST_PER_1K: f64 = 0.005;

impl Governor {
    pub fn new(budget: Budget, policy: CapabilityPolicy) -> Self {
        let mut g = Governor {
            budget,
            policy,
            usage: Usage { started_ms: chrono::Utc::now().timestamp_millis(), ..Default::default() },
            kill: KillSwitch::default(),
            audit: AuditChain::new(),
        };
        g.audit.append("run_start", "");
        g
    }

    pub fn checkpoint(&mut self, note: &str) -> Result<(), GovError> {
        if self.kill.tripped() {
            self.audit.append("aborted", &self.kill.reason);
            return Err(GovError::Aborted(self.kill.reason.clone()));
        }
        self.usage.steps += 1;
        if self.budget.max_steps > 0 && self.usage.steps > self.budget.max_steps {
            return Err(GovError::BudgetExceeded(format!("max_steps ({note})")));
        }
        let elapsed = (chrono::Utc::now().timestamp_millis() - self.usage.started_ms) as u64;
        if self.budget.max_wallclock_ms > 0 && elapsed > self.budget.max_wallclock_ms {
            return Err(GovError::BudgetExceeded("max_wallclock_ms".into()));
        }
        Ok(())
    }

    pub fn check_model(&mut self, model: &str) -> Result<(), GovError> {
        if !self.policy.allowed_models.is_empty() && !self.policy.allowed_models.contains(model) {
            self.audit.append("policy_violation", model);
            return Err(GovError::PolicyViolation(format!("model {model}")));
        }
        if self.budget.max_model_calls > 0 && self.usage.model_calls >= self.budget.max_model_calls {
            return Err(GovError::BudgetExceeded("max_model_calls".into()));
        }
        Ok(())
    }

    pub fn check_tool(&mut self, tool: &str) -> Result<(), GovError> {
        if self.policy.denied_tools.contains(tool)
            || (!self.policy.allowed_tools.is_empty() && !self.policy.allowed_tools.contains(tool))
        {
            self.audit.append("policy_violation", tool);
            return Err(GovError::PolicyViolation(format!("tool {tool}")));
        }
        Ok(())
    }

    pub fn record_call(&mut self, model: &str, tokens: u64) -> Result<(), GovError> {
        self.usage.model_calls += 1;
        self.usage.tokens += tokens;
        self.usage.cost_usd += (tokens as f64 / 1000.0) * COST_PER_1K;
        self.audit.append("model_call", &format!("{model}:{tokens}"));
        if self.budget.max_tokens > 0 && self.usage.tokens > self.budget.max_tokens {
            return Err(GovError::BudgetExceeded("max_tokens".into()));
        }
        if self.budget.max_cost_usd > 0.0 && self.usage.cost_usd > self.budget.max_cost_usd {
            return Err(GovError::BudgetExceeded("max_cost_usd".into()));
        }
        Ok(())
    }

    pub fn parallelism(&self, requested: u32) -> u32 {
        requested
            .min(self.budget.max_parallel)
            .min(if self.policy.max_agents == 0 { u32::MAX } else { self.policy.max_agents })
            .max(1)
    }
}

/// A thread-safe handle other kernel services share to enforce one run's limits.
pub type SharedGovernor = Arc<RwLock<Governor>>;

pub fn shared(budget: Budget, policy: CapabilityPolicy) -> SharedGovernor {
    Arc::new(RwLock::new(Governor::new(budget, policy)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_and_audit() {
        let mut g = Governor::new(
            Budget { max_model_calls: 2, ..Default::default() },
            CapabilityPolicy::default(),
        );
        assert!(g.check_model("m").is_ok());
        g.record_call("m", 100).unwrap();
        g.record_call("m", 100).unwrap();
        assert!(matches!(g.check_model("m"), Err(GovError::BudgetExceeded(_))));
        assert!(g.audit.verify());
    }

    #[test]
    fn kill_switch_aborts() {
        let mut g = Governor::new(Budget::default(), CapabilityPolicy::default());
        g.kill.trip("stop");
        assert!(matches!(g.checkpoint("x"), Err(GovError::Aborted(_))));
    }

    #[test]
    fn policy_denies() {
        let mut policy = CapabilityPolicy::default();
        policy.allowed_models.insert("ok".to_string());
        let mut g = Governor::new(Budget::default(), policy);
        assert!(g.check_model("ok").is_ok());
        assert!(matches!(g.check_model("nope"), Err(GovError::PolicyViolation(_))));
    }
}
