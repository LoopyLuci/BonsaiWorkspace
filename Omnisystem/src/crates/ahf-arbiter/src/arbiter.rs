//! Main Arbiter orchestrator combining decision engine and safety envelopes
//!
//! Coordinates the full arbiter flow: validation, decision making, and safety envelope application.

use crate::{
    AhfDecision, AhfError, AhfSignals, ConfidenceScore, Decision, DecisionEngine, PolicyEnforcer,
    SafetyEnvelope,
};
use ahf_core::{BiasScore, Criticality, GroundingScore, VerificationResult};
use chrono::Utc;
use std::time::Instant;
use tracing::{info, span, Level};
use uuid::Uuid;

/// Main Arbiter orchestrator
#[derive(Debug, Clone)]
pub struct Arbiter {
    decision_engine: DecisionEngine,
    policy_enforcer: PolicyEnforcer,
    safety_envelope: SafetyEnvelope,
    session_id: Uuid,
}

impl Arbiter {
    /// Create a new Arbiter with default policies and decision engine
    pub fn new() -> Result<Self, AhfError> {
        Ok(Arbiter {
            decision_engine: DecisionEngine::new(),
            policy_enforcer: PolicyEnforcer::new(),
            safety_envelope: SafetyEnvelope::new()?,
            session_id: Uuid::new_v4(),
        })
    }

    /// Create with custom policy enforcer
    pub fn with_enforcer(enforcer: PolicyEnforcer) -> Result<Self, AhfError> {
        Ok(Arbiter {
            decision_engine: enforcer.create_engine()?,
            policy_enforcer: enforcer,
            safety_envelope: SafetyEnvelope::new()?,
            session_id: Uuid::new_v4(),
        })
    }

    /// Get the current session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Make a decision and optionally apply safety envelope
    ///
    /// # Flow
    ///
    /// 1. Validate input signals
    /// 2. Apply decision engine rules
    /// 3. Apply safety envelope if decision is ACCEPT
    /// 4. Return decision with audit trail
    pub async fn arbitrate(
        &mut self,
        signals: &AhfSignals,
        output_text: Option<&str>,
        criticality: Criticality,
    ) -> Result<(AhfDecision, Option<String>), AhfError> {
        let span = span!(Level::INFO, "arbitrate", session_id = %self.session_id);
        let _guard = span.enter();

        let start = Instant::now();

        // Step 1: Validate input signals
        if !signals.validate() {
            return Err(AhfError::arbiter_decision_failed(
                "Input signals out of valid range",
            ));
        }

        info!("Starting arbitration process");

        // Step 2: Make decision
        let mut decision = self.decision_engine.decide(
            signals.grounding_score,
            &signals.verification_result,
            signals.model_confidence,
            signals.bias_score,
            criticality,
        )?;

        // Mark when safety envelope was considered
        decision.safety_envelope_applied = false;

        // Step 3: Apply safety envelope if ACCEPT and output provided
        let safe_output = if decision.decision == Decision::Accept && output_text.is_some() {
            let output = output_text.unwrap();

            // Apply envelope clamping (before validation)
            let safe_text = self.safety_envelope.apply(output)?;

            // Validate the clamped output
            if !self.safety_envelope.validate(&safe_text)? {
                info!("Output failed safety validation after clamping, decision changed to REJECT");
                decision.decision = Decision::Reject;
                decision.reason = ahf_core::DecisionReason::Custom(
                    "Output failed safety validation".to_string(),
                );
                decision.explanation =
                    "Output contained harmful phrases or exceeded safety constraints".to_string();
                None
            } else {
                decision.safety_envelope_applied = true;

                info!(
                    original_len = output.len(),
                    final_len = safe_text.len(),
                    "Safety envelope applied"
                );

                Some(safe_text)
            }
        } else if decision.decision == Decision::Accept {
            // Accept without output text
            info!("Decision: ACCEPT (no output text to clamp)");
            None
        } else {
            None
        };

        let latency = start.elapsed().as_millis() as f64;
        info!(
            decision = %decision.decision,
            latency_ms = latency,
            safety_applied = decision.safety_envelope_applied,
            "Arbitration complete"
        );

        Ok((decision, safe_output))
    }

    /// Update policy (council action)
    pub fn update_policy(
        &mut self,
        policy: ahf_core::ArbiterPolicy,
        set_by: &str,
    ) -> Result<(), AhfError> {
        self.policy_enforcer.set_policy(policy, set_by)?;
        self.decision_engine = self.policy_enforcer.create_engine()?;
        Ok(())
    }

    /// Get policy for a specific model
    pub fn get_model_policy(&self, model_name: &str) -> Result<ahf_core::ArbiterPolicy, AhfError> {
        self.policy_enforcer.get_model_policy(model_name)
    }

    /// Set model-specific policy
    pub fn set_model_policy(
        &mut self,
        model_policy: ahf_core::ModelPolicy,
    ) -> Result<(), AhfError> {
        self.policy_enforcer.set_model_policy(model_policy)?;
        Ok(())
    }

    /// Create model-specific decision engine
    pub fn create_model_engine(&self, model_name: &str) -> Result<DecisionEngine, AhfError> {
        self.policy_enforcer.create_model_engine(model_name)
    }

    /// Get current policy
    pub fn current_policy(&self) -> Result<ahf_core::ArbiterPolicy, AhfError> {
        self.policy_enforcer.get_policy()
    }

    /// Get policy change history
    pub fn policy_history(
        &self,
        threshold_name: &str,
    ) -> Result<Vec<ahf_core::PolicyVersion>, AhfError> {
        self.policy_enforcer.get_policy_history(threshold_name)
    }
}

impl Default for Arbiter {
    fn default() -> Self {
        Self::new().expect("Failed to create default Arbiter")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahf_core::{VerificationStatus, VerificationResult};

    fn make_signals(
        grounding: f64,
        confidence: f64,
        bias: f64,
    ) -> AhfSignals {
        AhfSignals {
            grounding_score: GroundingScore::new(
                (grounding * 10.0) as usize,
                10,
            ),
            verification_result: VerificationResult {
                status: VerificationStatus::Valid,
                proof: None,
                reasoning: "test".to_string(),
                confidence: 0.95,
            },
            model_confidence: ConfidenceScore::new(confidence),
            bias_score: BiasScore::new(bias, bias, bias, bias, bias),
        }
    }

    #[tokio::test]
    async fn test_arbiter_accept_no_output() {
        let mut arbiter = Arbiter::new().unwrap();
        let signals = make_signals(0.8, 0.85, 0.2);

        let (decision, output) = arbiter
            .arbitrate(&signals, None, Criticality::Low)
            .await
            .unwrap();

        assert_eq!(decision.decision, Decision::Accept);
        assert!(output.is_none());
    }

    #[tokio::test]
    async fn test_arbiter_accept_with_output() {
        let mut arbiter = Arbiter::new().unwrap();
        let signals = make_signals(0.8, 0.85, 0.2);
        let output = "The capital of France is Paris.";

        let (decision, safe_output) = arbiter
            .arbitrate(&signals, Some(output), Criticality::Low)
            .await
            .unwrap();

        assert_eq!(decision.decision, Decision::Accept);
        assert!(safe_output.is_some());
        assert_eq!(safe_output.unwrap(), output);
    }

    #[tokio::test]
    async fn test_arbiter_reject_low_grounding() {
        let mut arbiter = Arbiter::new().unwrap();
        let signals = make_signals(0.3, 0.85, 0.2);

        let (decision, output) = arbiter
            .arbitrate(&signals, Some("Some output"), Criticality::Low)
            .await
            .unwrap();

        assert_eq!(decision.decision, Decision::Reject);
        assert!(output.is_none());
    }

    #[tokio::test]
    async fn test_arbiter_with_certainty_expression() {
        let mut arbiter = Arbiter::new().unwrap();
        let signals = make_signals(0.8, 0.85, 0.2);
        let output = "I am certain that this fact is true.";

        let (decision, safe_output) = arbiter
            .arbitrate(&signals, Some(output), Criticality::Low)
            .await
            .unwrap();

        // Certainty expression should be replaced
        assert_eq!(decision.decision, Decision::Accept);
        assert!(!safe_output.unwrap().contains("I am certain that"));
    }

    #[tokio::test]
    async fn test_arbiter_escalate_high_criticality() {
        let mut arbiter = Arbiter::new().unwrap();
        let signals = make_signals(0.8, 0.85, 0.2);

        let (decision, _) = arbiter
            .arbitrate(&signals, Some("output"), Criticality::High)
            .await
            .unwrap();

        assert_eq!(decision.decision, Decision::Escalate);
    }

    #[tokio::test]
    async fn test_policy_update() {
        let mut arbiter = Arbiter::new().unwrap();
        let mut new_policy = ahf_core::ArbiterPolicy::default();
        new_policy.grounding_threshold = 0.9;

        arbiter.update_policy(new_policy, "test_council").unwrap();

        let policy = arbiter.current_policy().unwrap();
        assert_eq!(policy.grounding_threshold, 0.9);
    }

    #[tokio::test]
    async fn test_model_policy_setting() {
        let mut arbiter = Arbiter::new().unwrap();
        let model_policy = ahf_core::ModelPolicy {
            model_name: "test-model".to_string(),
            grounding_threshold: Some(0.85),
            confidence_threshold: None,
            bias_threshold: None,
        };

        arbiter.set_model_policy(model_policy).unwrap();

        let policy = arbiter.get_model_policy("test-model").unwrap();
        assert_eq!(policy.grounding_threshold, 0.85);
    }

    #[tokio::test]
    async fn test_safety_envelope_clamping() {
        let mut arbiter = Arbiter::new().unwrap();
        let signals = make_signals(0.8, 0.85, 0.2);
        let output = "I am absolutely confident that 150% is correct.";

        let (decision, safe_output) = arbiter
            .arbitrate(&signals, Some(output), Criticality::Low)
            .await
            .unwrap();

        let safe = safe_output.unwrap();
        assert!(!safe.contains("absolutely confident"));
        assert!(!safe.contains("150%")); // Clamped to 100%
    }

    #[tokio::test]
    async fn test_session_id_persistence() {
        let arbiter = Arbiter::new().unwrap();
        let id1 = arbiter.session_id();
        let id2 = arbiter.session_id();
        assert_eq!(id1, id2);
    }
}
