//! Formal decision logic for the Arbiter
//!
//! Implements the formally verified decision rules with comprehensive
//! reasoning and audit trail generation.

use crate::{AhfDecision, AhfError, ConfidenceScore, Decision, DecisionReason, DecisionSignals};
use ahf_core::{BiasScore, Criticality, GroundingScore, VerificationResult, VerificationStatus};
use chrono::Utc;
use std::time::Instant;
use tracing::{debug, info, span, Level};
use uuid::Uuid;

/// Core decision engine implementing formal AHF rules
#[derive(Debug, Clone)]
pub struct DecisionEngine {
    /// Thresholds for decision making
    pub grounding_threshold: f64,
    pub confidence_threshold: f64,
    pub bias_threshold: f64,
    /// Whether to measure performance
    pub measure_performance: bool,
}

impl DecisionEngine {
    /// Create a new decision engine with default thresholds
    pub fn new() -> Self {
        DecisionEngine {
            grounding_threshold: 0.7,
            confidence_threshold: 0.6,
            bias_threshold: 0.5,
            measure_performance: true,
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(
        grounding_threshold: f64,
        confidence_threshold: f64,
        bias_threshold: f64,
    ) -> Self {
        DecisionEngine {
            grounding_threshold: grounding_threshold.clamp(0.0, 1.0),
            confidence_threshold: confidence_threshold.clamp(0.0, 1.0),
            bias_threshold: bias_threshold.clamp(0.0, 1.0),
            measure_performance: true,
        }
    }

    /// Make a decision based on input signals
    ///
    /// # Rules (applied in order)
    ///
    /// 1. If verification_result == Invalid → REJECT
    /// 2. If grounding_score < THRESHOLD → REJECT
    /// 3. If model_confidence < MIN_CONFIDENCE → REJECT
    /// 4. If bias_score > BIAS_THRESHOLD → REJECT
    /// 5. If criticality == High && grounding_score < 1.0 → ESCALATE
    /// 6. Otherwise → ACCEPT
    pub fn decide(
        &self,
        grounding_score: GroundingScore,
        verification_result: &VerificationResult,
        model_confidence: ConfidenceScore,
        bias_score: BiasScore,
        criticality: Criticality,
    ) -> Result<AhfDecision, AhfError> {
        let span = span!(Level::DEBUG, "arbiter_decision");
        let _guard = span.enter();

        let start = Instant::now();

        debug!(
            grounding = grounding_score.score(),
            confidence = model_confidence.0,
            bias = bias_score.score,
            criticality = ?criticality,
            "Starting arbiter decision"
        );

        // Rule 1: Verification must be valid
        if verification_result.status == VerificationStatus::Invalid {
            let signals = DecisionSignals {
                grounding_score: grounding_score.score(),
                verification_valid: false,
                model_confidence: model_confidence.0,
                bias_score: bias_score.score,
                criticality_level: format!("{:?}", criticality),
            };

            let latency = start.elapsed().as_millis() as f64;
            let decision = AhfDecision::new(
                Decision::Reject,
                DecisionReason::VerificationFailed,
                "Verification found contradictions with knowledge sources".to_string(),
                signals,
                false,
            );

            info!(
                latency_ms = latency,
                reason = "verification_failed",
                "Decision: REJECT"
            );
            return Ok(decision);
        }

        // Rule 2: Grounding score must meet threshold
        if grounding_score.score() < self.grounding_threshold {
            let signals = DecisionSignals {
                grounding_score: grounding_score.score(),
                verification_valid: true,
                model_confidence: model_confidence.0,
                bias_score: bias_score.score,
                criticality_level: format!("{:?}", criticality),
            };

            let latency = start.elapsed().as_millis() as f64;
            let decision = AhfDecision::new(
                Decision::Reject,
                DecisionReason::LowGroundingScore,
                format!(
                    "Grounding score {:.2} below threshold {:.2}",
                    grounding_score.score(), self.grounding_threshold
                ),
                signals,
                false,
            );

            info!(
                latency_ms = latency,
                grounding_score = grounding_score.score(),
                threshold = self.grounding_threshold,
                reason = "low_grounding_score",
                "Decision: REJECT"
            );
            return Ok(decision);
        }

        // Rule 3: Model confidence must meet threshold
        if model_confidence.0 < self.confidence_threshold {
            let signals = DecisionSignals {
                grounding_score: grounding_score.score(),
                verification_valid: true,
                model_confidence: model_confidence.0,
                bias_score: bias_score.score,
                criticality_level: format!("{:?}", criticality),
            };

            let latency = start.elapsed().as_millis() as f64;
            let decision = AhfDecision::new(
                Decision::Reject,
                DecisionReason::LowConfidence,
                format!(
                    "Model confidence {:.2} below threshold {:.2}",
                    model_confidence.0, self.confidence_threshold
                ),
                signals,
                false,
            );

            info!(
                latency_ms = latency,
                model_confidence = model_confidence.0,
                threshold = self.confidence_threshold,
                reason = "low_confidence",
                "Decision: REJECT"
            );
            return Ok(decision);
        }

        // Rule 4: Bias must be below threshold
        if bias_score.score > self.bias_threshold {
            let signals = DecisionSignals {
                grounding_score: grounding_score.score(),
                verification_valid: true,
                model_confidence: model_confidence.0,
                bias_score: bias_score.score,
                criticality_level: format!("{:?}", criticality),
            };

            let latency = start.elapsed().as_millis() as f64;
            let decision = AhfDecision::new(
                Decision::Reject,
                DecisionReason::HighBias,
                format!(
                    "Bias score {:.2} exceeds threshold {:.2}",
                    bias_score.score, self.bias_threshold
                ),
                signals,
                false,
            );

            info!(
                latency_ms = latency,
                bias_score = bias_score.score,
                threshold = self.bias_threshold,
                reason = "high_bias",
                "Decision: REJECT"
            );
            return Ok(decision);
        }

        // Rule 5: High criticality with incomplete grounding → ESCALATE
        if criticality == Criticality::High && grounding_score.score() < 1.0 {
            let signals = DecisionSignals {
                grounding_score: grounding_score.score(),
                verification_valid: true,
                model_confidence: model_confidence.0,
                bias_score: bias_score.score,
                criticality_level: format!("{:?}", criticality),
            };

            let latency = start.elapsed().as_millis() as f64;
            let decision = AhfDecision::new(
                Decision::Escalate,
                DecisionReason::HighCriticality,
                format!(
                    "High criticality claim with incomplete grounding ({:.2}). Requires human review.",
                    grounding_score.score()
                ),
                signals,
                false,
            );

            info!(
                latency_ms = latency,
                grounding_score = grounding_score.score(),
                criticality = ?criticality,
                reason = "high_criticality",
                "Decision: ESCALATE"
            );
            return Ok(decision);
        }

        // Rule 6: All checks passed → ACCEPT
        let signals = DecisionSignals {
            grounding_score: grounding_score.score(),
            verification_valid: true,
            model_confidence: model_confidence.0,
            bias_score: bias_score.score,
            criticality_level: format!("{:?}", criticality),
        };

        let latency = start.elapsed().as_millis() as f64;
        let decision = AhfDecision::new(
            Decision::Accept,
            DecisionReason::AllChecksPassed,
            "All verification checks passed. Output approved for delivery.".to_string(),
            signals,
            false,
        );

        info!(
            latency_ms = latency,
            grounding_score = grounding_score.score(),
            model_confidence = model_confidence.0,
            bias_score = bias_score.score,
            reason = "all_checks_passed",
            "Decision: ACCEPT"
        );

        Ok(decision)
    }

    /// Update decision thresholds
    pub fn update_thresholds(
        &mut self,
        grounding_threshold: Option<f64>,
        confidence_threshold: Option<f64>,
        bias_threshold: Option<f64>,
    ) -> Result<(), AhfError> {
        if let Some(gt) = grounding_threshold {
            if gt < 0.0 || gt > 1.0 {
                return Err(AhfError::invalid_configuration(
                    "Grounding threshold must be in [0.0, 1.0]",
                ));
            }
            self.grounding_threshold = gt;
        }

        if let Some(ct) = confidence_threshold {
            if ct < 0.0 || ct > 1.0 {
                return Err(AhfError::invalid_configuration(
                    "Confidence threshold must be in [0.0, 1.0]",
                ));
            }
            self.confidence_threshold = ct;
        }

        if let Some(bt) = bias_threshold {
            if bt < 0.0 || bt > 1.0 {
                return Err(AhfError::invalid_configuration(
                    "Bias threshold must be in [0.0, 1.0]",
                ));
            }
            self.bias_threshold = bt;
        }

        Ok(())
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahf_core::VerificationStatus;

    fn make_verification_result(status: VerificationStatus) -> VerificationResult {
        VerificationResult {
            status,
            proof: None,
            reasoning: "test".to_string(),
            confidence: 0.95,
        }
    }

    #[test]
    fn test_rule_1_verification_invalid() {
        let engine = DecisionEngine::new();
        let grounding = GroundingScore::new(5, 5);
        let verification = make_verification_result(VerificationStatus::Invalid);
        let confidence = ConfidenceScore::new(0.9);
        let bias = BiasScore::clean();

        let decision = engine
            .decide(grounding, &verification, confidence, bias, Criticality::Low)
            .unwrap();

        assert_eq!(decision.decision, Decision::Reject);
        assert_eq!(decision.reason, DecisionReason::VerificationFailed);
    }

    #[test]
    fn test_rule_2_low_grounding_score() {
        let engine = DecisionEngine::new();
        let grounding = GroundingScore::new(2, 10); // 0.2 < 0.7 threshold
        let verification = make_verification_result(VerificationStatus::Valid);
        let confidence = ConfidenceScore::new(0.9);
        let bias = BiasScore::clean();

        let decision = engine
            .decide(grounding, &verification, confidence, bias, Criticality::Low)
            .unwrap();

        assert_eq!(decision.decision, Decision::Reject);
        assert_eq!(decision.reason, DecisionReason::LowGroundingScore);
    }

    #[test]
    fn test_rule_3_low_confidence() {
        let engine = DecisionEngine::new();
        let grounding = GroundingScore::new(8, 10);
        let verification = make_verification_result(VerificationStatus::Valid);
        let confidence = ConfidenceScore::new(0.3); // 0.3 < 0.6 threshold
        let bias = BiasScore::clean();

        let decision = engine
            .decide(grounding, &verification, confidence, bias, Criticality::Low)
            .unwrap();

        assert_eq!(decision.decision, Decision::Reject);
        assert_eq!(decision.reason, DecisionReason::LowConfidence);
    }

    #[test]
    fn test_rule_4_high_bias() {
        let engine = DecisionEngine::new();
        let grounding = GroundingScore::new(8, 10);
        let verification = make_verification_result(VerificationStatus::Valid);
        let confidence = ConfidenceScore::new(0.9);
        let bias = BiasScore::new(0.9, 0.9, 0.9, 0.9, 0.9); // score = 0.9 > 0.5 threshold

        let decision = engine
            .decide(grounding, &verification, confidence, bias, Criticality::Low)
            .unwrap();

        assert_eq!(decision.decision, Decision::Reject);
        assert_eq!(decision.reason, DecisionReason::HighBias);
    }

    #[test]
    fn test_rule_5_high_criticality_escalate() {
        let engine = DecisionEngine::new();
        let grounding = GroundingScore::new(8, 10); // 0.8 < 1.0, so escalates
        let verification = make_verification_result(VerificationStatus::Valid);
        let confidence = ConfidenceScore::new(0.9);
        let bias = BiasScore::clean();

        let decision = engine
            .decide(
                grounding,
                &verification,
                confidence,
                bias,
                Criticality::High,
            )
            .unwrap();

        assert_eq!(decision.decision, Decision::Escalate);
        assert_eq!(decision.reason, DecisionReason::HighCriticality);
    }

    #[test]
    fn test_rule_6_all_checks_passed() {
        let engine = DecisionEngine::new();
        let grounding = GroundingScore::new(8, 10);
        let verification = make_verification_result(VerificationStatus::Valid);
        let confidence = ConfidenceScore::new(0.9);
        let bias = BiasScore::clean();

        let decision = engine
            .decide(
                grounding,
                &verification,
                confidence,
                bias,
                Criticality::Low,
            )
            .unwrap();

        assert_eq!(decision.decision, Decision::Accept);
        assert_eq!(decision.reason, DecisionReason::AllChecksPassed);
    }

    #[test]
    fn test_custom_thresholds() {
        let engine = DecisionEngine::with_thresholds(0.9, 0.8, 0.4);
        assert_eq!(engine.grounding_threshold, 0.9);
        assert_eq!(engine.confidence_threshold, 0.8);
        assert_eq!(engine.bias_threshold, 0.4);
    }

    #[test]
    fn test_update_thresholds() {
        let mut engine = DecisionEngine::new();
        engine
            .update_thresholds(Some(0.8), Some(0.7), Some(0.4))
            .unwrap();

        assert_eq!(engine.grounding_threshold, 0.8);
        assert_eq!(engine.confidence_threshold, 0.7);
        assert_eq!(engine.bias_threshold, 0.4);
    }

    #[test]
    fn test_invalid_threshold() {
        let mut engine = DecisionEngine::new();
        let result = engine.update_thresholds(Some(1.5), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_high_criticality_with_perfect_grounding() {
        let engine = DecisionEngine::new();
        let grounding = GroundingScore::new(10, 10); // 1.0 grounding
        let verification = make_verification_result(VerificationStatus::Valid);
        let confidence = ConfidenceScore::new(0.9);
        let bias = BiasScore::clean();

        let decision = engine
            .decide(
                grounding,
                &verification,
                confidence,
                bias,
                Criticality::High,
            )
            .unwrap();

        // High criticality with perfect grounding still accepts
        assert_eq!(decision.decision, Decision::Accept);
    }

    #[test]
    fn test_decision_signals_captured() {
        let engine = DecisionEngine::new();
        let grounding = GroundingScore::new(7, 10);
        let verification = make_verification_result(VerificationStatus::Valid);
        let confidence = ConfidenceScore::new(0.85);
        let bias = BiasScore::new(0.2, 0.1, 0.15, 0.1, 0.2);

        let decision = engine
            .decide(grounding, &verification, confidence, bias, Criticality::Medium)
            .unwrap();

        assert!((decision.signals.grounding_score - 0.7).abs() < f64::EPSILON);
        assert!((decision.signals.model_confidence - 0.85).abs() < f64::EPSILON);
        assert!((decision.signals.bias_score - bias.score).abs() < f64::EPSILON);
    }
}
