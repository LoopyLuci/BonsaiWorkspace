//! Decision types and audit logging for the Anti-Hallucination Framework

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// The decision made by the Arbiter
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Decision {
    /// Accept the output
    Accept,
    /// Reject the output
    Reject,
    /// Escalate to human reviewer
    Escalate,
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decision::Accept => write!(f, "ACCEPT"),
            Decision::Reject => write!(f, "REJECT"),
            Decision::Escalate => write!(f, "ESCALATE"),
        }
    }
}

/// Reason for the decision
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecisionReason {
    /// Verification failed - claim contradicted
    VerificationFailed,
    /// Grounding score too low
    LowGroundingScore,
    /// Model confidence too low
    LowConfidence,
    /// Bias score too high
    HighBias,
    /// High criticality requires escalation
    HighCriticality,
    /// All checks passed
    AllChecksPassed,
    /// Custom reason
    Custom(String),
}

impl std::fmt::Display for DecisionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionReason::VerificationFailed => write!(f, "verification_failed"),
            DecisionReason::LowGroundingScore => write!(f, "low_grounding_score"),
            DecisionReason::LowConfidence => write!(f, "low_confidence"),
            DecisionReason::HighBias => write!(f, "high_bias"),
            DecisionReason::HighCriticality => write!(f, "high_criticality"),
            DecisionReason::AllChecksPassed => write!(f, "all_checks_passed"),
            DecisionReason::Custom(reason) => write!(f, "custom: {}", reason),
        }
    }
}

/// Complete decision with reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhfDecision {
    /// Unique identifier
    pub id: Uuid,
    /// The decision itself
    pub decision: Decision,
    /// Primary reason
    pub reason: DecisionReason,
    /// Detailed explanation
    pub explanation: String,
    /// Timestamp of decision
    pub timestamp: DateTime<Utc>,
    /// Input signals used
    pub signals: DecisionSignals,
    /// Whether safety envelopes were applied
    pub safety_envelope_applied: bool,
}

/// Signals used in the decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSignals {
    pub grounding_score: f64,
    pub verification_valid: bool,
    pub model_confidence: f64,
    pub bias_score: f64,
    pub criticality_level: String,
}

impl AhfDecision {
    /// Create a new decision
    pub fn new(
        decision: Decision,
        reason: DecisionReason,
        explanation: String,
        signals: DecisionSignals,
        safety_envelope_applied: bool,
    ) -> Self {
        AhfDecision {
            id: Uuid::new_v4(),
            decision,
            reason,
            explanation,
            timestamp: Utc::now(),
            signals,
            safety_envelope_applied,
        }
    }
}

/// Decision log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionLog {
    /// Timestamp of decision
    pub timestamp: DateTime<Utc>,
    /// The decision made
    pub decision: AhfDecision,
    /// Optional session/context ID
    pub session_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_display() {
        assert_eq!(Decision::Accept.to_string(), "ACCEPT");
        assert_eq!(Decision::Reject.to_string(), "REJECT");
        assert_eq!(Decision::Escalate.to_string(), "ESCALATE");
    }

    #[test]
    fn test_decision_reason_display() {
        assert_eq!(DecisionReason::VerificationFailed.to_string(), "verification_failed");
        assert_eq!(DecisionReason::HighBias.to_string(), "high_bias");
    }

    #[test]
    fn test_ahf_decision_creation() {
        let signals = DecisionSignals {
            grounding_score: 0.85,
            verification_valid: true,
            model_confidence: 0.90,
            bias_score: 0.1,
            criticality_level: "medium".to_string(),
        };

        let decision = AhfDecision::new(
            Decision::Accept,
            DecisionReason::AllChecksPassed,
            "All verification checks passed".to_string(),
            signals,
            false,
        );

        assert_eq!(decision.decision, Decision::Accept);
        assert!(!decision.safety_envelope_applied);
    }
}
