//! UCR (Unified Capability Registry) Effect System
//!
//! Every observable side-effect a Bonsai capability can produce is expressed as
//! a `BonsaiEffect` variant.  The `TrustGuard` validates effect sequences against
//! a per-capability policy before they are dispatched.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Effect type hierarchy ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BonsaiEffect {
    // ── I/O effects ──────────────────────────────────────────────────────────
    ReadFile       { path: String },
    WriteFile      { path: String, bytes: u64 },
    DeleteFile     { path: String },
    ListDir        { path: String },
    NetworkFetch   { url: String, method: String },
    NetworkPost    { url: String },
    RunProcess     { cmd: String, args: Vec<String> },
    ReadEnv        { key: String },
    WriteEnv       { key: String },

    // ── Model effects ────────────────────────────────────────────────────────
    ModelInference { model_id: String, tokens: u32 },
    AdapterSwitch  { from: Option<String>, to: String },
    TrainingIngest { domain: String, example_count: u32 },

    // ── UI effects ───────────────────────────────────────────────────────────
    EmitEvent      { event: String },
    ShowNotification { title: String },
    OpenWindow     { label: String },

    // ── Memory effects ───────────────────────────────────────────────────────
    MemoryWrite    { namespace: String, key: String },
    MemoryDelete   { namespace: String, key: String },

    // ── Audit / meta ────────────────────────────────────────────────────────
    LogAudit       { message: String },
    PolicyViolation { policy: String, details: String },
}

// ── Trust levels ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Untrusted = 0,
    Sandboxed = 1,
    Trusted   = 2,
    System    = 3,
}

// ── Effect row (audit log entry) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectRow {
    pub id:           String,
    pub capability:   String,
    pub trust_level:  TrustLevel,
    pub effect:       BonsaiEffect,
    pub allowed:      bool,
    pub reason:       Option<String>,
    pub timestamp_ms: i64,
}

// ── Policy ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectPolicy {
    /// Which effects are explicitly allowed (checked by kind discriminant).
    pub allowed_kinds: Vec<String>,
    /// Maximum trust level for this capability.
    pub max_trust: TrustLevel,
    /// If true, all effects are denied unless explicitly listed.
    pub deny_by_default: bool,
}

impl EffectPolicy {
    pub fn permissive(max_trust: TrustLevel) -> Self {
        Self { allowed_kinds: vec![], max_trust, deny_by_default: false }
    }

    pub fn restrictive(allowed_kinds: Vec<&str>, max_trust: TrustLevel) -> Self {
        Self {
            allowed_kinds: allowed_kinds.into_iter().map(String::from).collect(),
            max_trust,
            deny_by_default: true,
        }
    }

    fn effect_kind(e: &BonsaiEffect) -> &'static str {
        match e {
            BonsaiEffect::ReadFile { .. }        => "read_file",
            BonsaiEffect::WriteFile { .. }       => "write_file",
            BonsaiEffect::DeleteFile { .. }      => "delete_file",
            BonsaiEffect::ListDir { .. }         => "list_dir",
            BonsaiEffect::NetworkFetch { .. }    => "network_fetch",
            BonsaiEffect::NetworkPost { .. }     => "network_post",
            BonsaiEffect::RunProcess { .. }      => "run_process",
            BonsaiEffect::ReadEnv { .. }         => "read_env",
            BonsaiEffect::WriteEnv { .. }        => "write_env",
            BonsaiEffect::ModelInference { .. }  => "model_inference",
            BonsaiEffect::AdapterSwitch { .. }   => "adapter_switch",
            BonsaiEffect::TrainingIngest { .. }  => "training_ingest",
            BonsaiEffect::EmitEvent { .. }       => "emit_event",
            BonsaiEffect::ShowNotification { .. } => "show_notification",
            BonsaiEffect::OpenWindow { .. }      => "open_window",
            BonsaiEffect::MemoryWrite { .. }     => "memory_write",
            BonsaiEffect::MemoryDelete { .. }    => "memory_delete",
            BonsaiEffect::LogAudit { .. }        => "log_audit",
            BonsaiEffect::PolicyViolation { .. } => "policy_violation",
        }
    }

    pub fn check(&self, effect: &BonsaiEffect, trust: TrustLevel) -> (bool, Option<String>) {
        if trust > self.max_trust {
            return (false, Some(format!("trust level {:?} exceeds max {:?}", trust, self.max_trust)));
        }
        let kind = Self::effect_kind(effect);
        if self.deny_by_default {
            if self.allowed_kinds.iter().any(|k| k == kind) {
                (true, None)
            } else {
                (false, Some(format!("effect '{}' not in allow-list", kind)))
            }
        } else {
            (true, None)
        }
    }
}

// ── TrustGuard ────────────────────────────────────────────────────────────────

pub struct TrustGuard {
    policies: HashMap<String, EffectPolicy>,
    log:      std::sync::Mutex<Vec<EffectRow>>,
}

impl TrustGuard {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            log:      std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn register_policy(&mut self, capability: impl Into<String>, policy: EffectPolicy) {
        self.policies.insert(capability.into(), policy);
    }

    /// Evaluate `effect` for `capability` at `trust`.  Returns `Ok(())` if
    /// allowed, `Err(reason)` if denied.  Always appends to the audit log.
    pub fn evaluate(
        &self,
        capability: &str,
        trust: TrustLevel,
        effect: BonsaiEffect,
    ) -> Result<(), String> {
        let policy = self.policies.get(capability);
        let (allowed, reason) = match policy {
            Some(p) => p.check(&effect, trust),
            None    => (true, None), // no policy = permissive
        };

        let row = EffectRow {
            id:           uuid_v4(),
            capability:   capability.to_string(),
            trust_level:  trust,
            effect,
            allowed,
            reason:       reason.clone(),
            timestamp_ms: now_ms(),
        };

        if let Ok(mut log) = self.log.lock() {
            log.push(row);
            // Rolling window: keep latest 10 000 entries
            if log.len() > 10_000 { log.drain(..1_000); }
        }

        if allowed { Ok(()) } else { Err(reason.unwrap_or_else(|| "denied".into())) }
    }

    /// Return the last `n` audit rows for `capability` (or all if `None`).
    pub fn audit_tail(&self, capability: Option<&str>, n: usize) -> Vec<EffectRow> {
        let log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        let filtered: Vec<_> = log.iter()
            .filter(|r| capability.map_or(true, |c| r.capability == c))
            .cloned()
            .collect();
        let start = filtered.len().saturating_sub(n);
        filtered[start..].to_vec()
    }
}

impl Default for TrustGuard {
    fn default() -> Self { Self::new() }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    format!("{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}", t, t >> 16, t & 0xfff, 0x8000 | (t & 0x3fff), t as u64 * 0x1000193)
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}
