//! Message types for the Anti-Hallucination Gateway

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use ahf_core::{AhfDecision, Criticality};

/// Request to verify a model output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    /// Unique request ID
    pub id: Uuid,
    /// Model output text to verify
    pub output: String,
    /// Model identifier
    pub model_id: String,
    /// Criticality level
    pub criticality: Criticality,
    /// Optional user ID for audit trail
    pub user_id: Option<String>,
    /// Optional session ID for correlation
    pub session_id: Option<Uuid>,
    /// Timestamp of request
    pub timestamp: DateTime<Utc>,
}

impl VerifyRequest {
    pub fn new(output: String, model_id: String, criticality: Criticality) -> Self {
        Self {
            id: Uuid::new_v4(),
            output,
            model_id,
            criticality,
            user_id: None,
            session_id: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

/// Output from the model that needs verification (actor message)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOutput {
    /// Model output text
    pub output: String,
    /// Model identifier
    pub model_id: String,
    /// Criticality level
    pub criticality: Criticality,
}

/// Result of verification from the gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhfResult {
    /// The decision made by the arbiter
    pub decision: AhfDecision,
    /// Optional modified output (after safety envelopes)
    pub output: Option<String>,
    /// Optional fallback output (if decision is Reject)
    pub fallback: Option<String>,
    /// Optional escalation reason
    pub escalation: Option<String>,
    /// Time taken in milliseconds
    pub latency_ms: u64,
    /// Whether this was processed under time pressure
    pub timed_out: bool,
}

impl AhfResult {
    pub fn new(decision: AhfDecision) -> Self {
        Self {
            decision,
            output: None,
            fallback: None,
            escalation: None,
            latency_ms: 0,
            timed_out: false,
        }
    }

    pub fn with_output(mut self, output: String) -> Self {
        self.output = Some(output);
        self
    }

    pub fn with_fallback(mut self, fallback: String) -> Self {
        self.fallback = Some(fallback);
        self
    }

    pub fn with_escalation(mut self, reason: String) -> Self {
        self.escalation = Some(reason);
        self
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    pub fn with_timeout(mut self, timed_out: bool) -> Self {
        self.timed_out = timed_out;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_request_creation() {
        let req = VerifyRequest::new(
            "Test output".to_string(),
            "gpt-4".to_string(),
            Criticality::High,
        );
        assert_eq!(req.output, "Test output");
        assert_eq!(req.model_id, "gpt-4");
        assert_eq!(req.criticality, Criticality::High);
        assert!(req.user_id.is_none());
    }

    #[test]
    fn test_verify_request_builder() {
        let req = VerifyRequest::new(
            "Test output".to_string(),
            "gpt-4".to_string(),
            Criticality::Medium,
        )
        .with_user_id("user123".to_string())
        .with_session_id(Uuid::new_v4());

        assert_eq!(req.user_id, Some("user123".to_string()));
        assert!(req.session_id.is_some());
    }

    #[test]
    fn test_ahf_result_builder() {
        let signals = ahf_core::DecisionSignals {
            grounding_score: 0.85,
            verification_valid: true,
            model_confidence: 0.90,
            bias_score: 0.1,
            criticality_level: "high".to_string(),
        };

        let decision = ahf_core::AhfDecision::new(
            ahf_core::Decision::Accept,
            ahf_core::DecisionReason::AllChecksPassed,
            "All checks passed".to_string(),
            signals,
            false,
        );

        let result = AhfResult::new(decision)
            .with_output("verified output".to_string())
            .with_latency(25);

        assert_eq!(result.output, Some("verified output".to_string()));
        assert_eq!(result.latency_ms, 25);
        assert!(!result.timed_out);
    }
}
