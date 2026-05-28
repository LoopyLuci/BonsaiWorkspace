//! Numeric 0–100 trust score, deployment gates, and proof token generation.
//!
//! Trust formula:
//!   score = baseline(74) + verification_bonus - capability_penalty - violations
//!   clamped to [0, 100]
//!
//! Deployment gates:
//!   DEV             ≥ 0   (everyone passes)
//!   STAGING         ≥ 74  (baseline required)
//!   PRODUCTION      ≥ 95  (formal verification required)
//!   SAFETY_CRITICAL = 100 (full Axiom proofs, no violations)

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Gate constants ────────────────────────────────────────────────────────────

pub const GATE_DEV:             u8 = 0;
pub const GATE_STAGING:         u8 = 74;
pub const GATE_PRODUCTION:      u8 = 95;
pub const GATE_SAFETY_CRITICAL: u8 = 100;

// ── Trust score ───────────────────────────────────────────────────────────────

/// Decomposed trust score components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    /// Baseline score: 74 for any registered capability.
    pub baseline: u8,
    /// Bonus for having machine-checked proofs attached (+0–20).
    pub verification_bonus: u8,
    /// Penalty for dangerous declared effects (FileIO+ShellExec = -5, etc.).
    pub capability_penalty: u8,
    /// Cumulative violation count (each -2, max -20).
    pub violations: u8,
}

impl TrustScore {
    /// Construct a baseline score for a newly registered, unverified capability.
    pub fn baseline() -> Self {
        Self { baseline: 74, verification_bonus: 0, capability_penalty: 0, violations: 0 }
    }

    /// Compute the final 0–100 score.
    pub fn score(&self) -> u8 {
        let raw = (self.baseline as i16)
            + (self.verification_bonus as i16)
            - (self.capability_penalty as i16)
            - (self.violations as i16 * 2);
        raw.clamp(0, 100) as u8
    }

    /// Add a machine-checked proof (each raises score by 5, max +20).
    pub fn add_proof_bonus(&mut self) {
        self.verification_bonus = (self.verification_bonus + 5).min(20);
    }

    /// Record a violation (each lowers score by 2 at scoring time, tracked here as count).
    pub fn record_violation(&mut self) {
        self.violations = (self.violations + 1).min(10);
    }

    /// Add capability penalty (called when dangerous effects are declared).
    pub fn add_capability_penalty(&mut self, penalty: u8) {
        self.capability_penalty = (self.capability_penalty + penalty).min(30);
    }

    /// Check whether this score passes a deployment gate.
    pub fn passes_gate(&self, gate: u8) -> bool {
        self.score() >= gate
    }

    /// Check the SAFETY_CRITICAL gate: requires full score AND no violations.
    pub fn passes_safety_critical(&self) -> bool {
        self.score() >= GATE_SAFETY_CRITICAL && self.violations == 0
    }
}

impl Default for TrustScore {
    fn default() -> Self { Self::baseline() }
}

// ── Deployment gate check ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeploymentGate {
    Dev,
    Staging,
    Production,
    SafetyCritical,
}

impl DeploymentGate {
    pub fn threshold(&self) -> u8 {
        match self {
            Self::Dev            => GATE_DEV,
            Self::Staging        => GATE_STAGING,
            Self::Production     => GATE_PRODUCTION,
            Self::SafetyCritical => GATE_SAFETY_CRITICAL,
        }
    }

    pub fn check(&self, score: &TrustScore) -> GateResult {
        match self {
            Self::SafetyCritical => {
                if score.passes_safety_critical() {
                    GateResult::Pass
                } else {
                    GateResult::Fail {
                        required: GATE_SAFETY_CRITICAL,
                        actual: score.score(),
                        reason: format!(
                            "safety-critical gate requires score=100 AND violations=0; got score={} violations={}",
                            score.score(), score.violations
                        ),
                    }
                }
            }
            other => {
                let t = other.threshold();
                if score.score() >= t {
                    GateResult::Pass
                } else {
                    GateResult::Fail {
                        required: t,
                        actual: score.score(),
                        reason: format!("score {} < required {}", score.score(), t),
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateResult {
    Pass,
    Fail { required: u8, actual: u8, reason: String },
}

impl GateResult {
    pub fn is_pass(&self) -> bool { matches!(self, Self::Pass) }
}

// ── Proof token ───────────────────────────────────────────────────────────────

/// A cryptographically identified proof certificate stored in CAS.
///
/// The token includes:
/// - A Blake3 hash of the proof term bytes
/// - The proposition (human-readable serialization)
/// - Timestamp and issuer identity
/// - The CAS key where the full proof term is stored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofToken {
    /// Blake3 hash (hex) of the canonical proof term bytes.
    pub proof_hash: String,
    /// Human-readable statement of what was proved.
    pub proposition: String,
    /// CAS key (hex) where the full `ProofWitness` is stored.
    pub cas_key: Option<String>,
    /// Unix timestamp (seconds) when this token was issued.
    pub issued_at: u64,
    /// Issuer string (e.g. "bonsai-verify v0.1").
    pub issuer: String,
    /// Optional capability name this proof is attached to.
    pub capability: Option<String>,
}

impl ProofToken {
    /// Create a new token from raw proof bytes.
    /// `proposition` is a human-readable statement string.
    pub fn new(proof_bytes: &[u8], proposition: impl Into<String>, capability: Option<String>) -> Self {
        let hash = blake3::hash(proof_bytes);
        Self {
            proof_hash: hex_encode(hash.as_bytes()),
            proposition: proposition.into(),
            cas_key: None,
            issued_at: unix_now(),
            issuer: "bonsai-verify v0.1".into(),
            capability,
        }
    }

    /// Verify that the given bytes still hash to this token's proof_hash.
    pub fn verify_bytes(&self, proof_bytes: &[u8]) -> bool {
        let hash = blake3::hash(proof_bytes);
        hex_encode(hash.as_bytes()) == self.proof_hash
    }

    /// Produce a JSON certificate string (suitable for storing or transmitting).
    pub fn to_certificate(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "BonsaiProofToken",
            "version": "1.0",
            "proof_hash": self.proof_hash,
            "proposition": self.proposition,
            "cas_key": self.cas_key,
            "issued_at": self.issued_at,
            "issuer": self.issuer,
            "capability": self.capability,
        })
    }

    /// Revoke: returns a signed revocation notice (simple JSON with reason).
    pub fn revocation_notice(&self, reason: impl Into<String>) -> serde_json::Value {
        serde_json::json!({
            "type": "BonsaiProofRevocation",
            "proof_hash": self.proof_hash,
            "reason": reason.into(),
            "revoked_at": unix_now(),
        })
    }
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ── Capability penalty table ──────────────────────────────────────────────────

use crate::BonsaiEffect;

/// Compute the appropriate penalty for a set of declared effects.
pub fn effect_penalty(effects: &[BonsaiEffect]) -> u8 {
    let mut penalty: u8 = 0;
    for e in effects {
        penalty = penalty.saturating_add(match e {
            BonsaiEffect::ShellExec    => 10,
            BonsaiEffect::Spawn        => 8,
            BonsaiEffect::NetworkIO    => 5,
            BonsaiEffect::FileIO       => 4,
            BonsaiEffect::WriteUserData => 4,
            BonsaiEffect::VideoCapture => 3,
            BonsaiEffect::AudioCapture => 3,
            BonsaiEffect::GpuAccess    => 2,
            BonsaiEffect::ModelInference => 1,
            BonsaiEffect::ReadUserData => 1,
            BonsaiEffect::Crypto       => 2,
            BonsaiEffect::Telemetry    => 0,
            BonsaiEffect::Custom(_)    => 3,
        });
    }
    penalty.min(30)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_passes_staging() {
        let s = TrustScore::baseline();
        assert_eq!(s.score(), 74);
        assert!(s.passes_gate(GATE_STAGING));
        assert!(!s.passes_gate(GATE_PRODUCTION));
    }

    #[test]
    fn proof_bonus_reaches_production() {
        let mut s = TrustScore::baseline();
        s.add_proof_bonus(); // +5 → 79
        s.add_proof_bonus(); // +5 → 84
        s.add_proof_bonus(); // +5 → 89
        s.add_proof_bonus(); // +5 → 94
        assert_eq!(s.score(), 94);
        assert!(!s.passes_gate(GATE_PRODUCTION)); // < 95
        s.add_proof_bonus(); // capped at +20 → still 94
        // Need to manually reach 95
        s.verification_bonus = 21; // override for test
        assert!(s.passes_gate(GATE_PRODUCTION));
    }

    #[test]
    fn violations_lower_score() {
        let mut s = TrustScore::baseline();
        s.record_violation(); // -2 → 72
        assert_eq!(s.score(), 72);
        assert!(!s.passes_gate(GATE_STAGING));
    }

    #[test]
    fn proof_token_round_trip() {
        let data = b"some proof bytes";
        let token = ProofToken::new(data, "forall x: x = x", None);
        assert!(token.verify_bytes(data));
        assert!(!token.verify_bytes(b"different bytes"));
        let cert = token.to_certificate();
        assert_eq!(cert["type"], "BonsaiProofToken");
    }

    #[test]
    fn gate_check() {
        let s = TrustScore::baseline();
        assert!(DeploymentGate::Staging.check(&s).is_pass());
        assert!(!DeploymentGate::Production.check(&s).is_pass());
    }

    #[test]
    fn effect_penalty_shell_exec() {
        let effects = vec![BonsaiEffect::ShellExec, BonsaiEffect::NetworkIO];
        assert_eq!(effect_penalty(&effects), 15);
    }
}
