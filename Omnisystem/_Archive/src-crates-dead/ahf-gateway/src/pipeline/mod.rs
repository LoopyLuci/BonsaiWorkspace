//! AHF Pipeline - orchestrates all AHF components

use crate::error::{GatewayError, GatewayResult};
use crate::config::AhfConfig;
use ahf_core::{
    AhfDecision, BiasScore, ConfidenceScore, Criticality, Decision, DecisionReason,
    DecisionSignals, GroundingScore, VerificationResult, VerificationStatus,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Result of a pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    /// The final decision
    pub decision: AhfDecision,
    /// Processing time in milliseconds
    pub latency_ms: u64,
    /// Whether execution hit timeout
    pub timed_out: bool,
    /// Intermediate results for debugging
    pub debug_info: PipelineDebugInfo,
}

/// Debug information from pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDebugInfo {
    /// Extracted claims count
    pub claims_count: usize,
    /// Grounding score result
    pub grounding_score: f64,
    /// Verification status
    pub verification_valid: bool,
    /// Model confidence extracted
    pub model_confidence: f64,
    /// Bias score detected
    pub bias_score: f64,
    /// Time spent on KGS (ms)
    pub kgs_time_ms: u64,
    /// Time spent on verification (ms)
    pub verification_time_ms: u64,
    /// Time spent on bias detection (ms)
    pub bias_detection_time_ms: u64,
    /// Time spent on arbiter decision (ms)
    pub arbiter_time_ms: u64,
}

impl Default for PipelineDebugInfo {
    fn default() -> Self {
        Self {
            claims_count: 0,
            grounding_score: 0.0,
            verification_valid: false,
            model_confidence: 0.0,
            bias_score: 0.0,
            kgs_time_ms: 0,
            verification_time_ms: 0,
            bias_detection_time_ms: 0,
            arbiter_time_ms: 0,
        }
    }
}

/// The AHF Pipeline
pub struct AhfPipeline {
    config: AhfConfig,
}

impl AhfPipeline {
    pub fn new(config: AhfConfig) -> Self {
        Self { config }
    }

    /// Execute the full verification pipeline
    pub async fn verify(
        &self,
        output: &str,
        model_id: &str,
        criticality: Criticality,
    ) -> GatewayResult<PipelineResult> {
        let start = Instant::now();
        let mut debug_info = PipelineDebugInfo::default();

        // 1. Parse output & validate schema
        if output.is_empty() {
            return Err(GatewayError::invalid_input("Output cannot be empty"));
        }

        // 2. Extract factual claims (simulated)
        debug_info.claims_count = self.count_sentences(output);

        // 3. Spawn parallel tasks: KGS lookup + Formal verification
        let kgs_start = Instant::now();
        let grounding_score = self.simulate_kgs_lookup(output).await;
        debug_info.grounding_score = grounding_score;
        debug_info.kgs_time_ms = kgs_start.elapsed().as_millis() as u64;

        let verification_start = Instant::now();
        let verification_valid = self.simulate_verification(output).await;
        debug_info.verification_valid = verification_valid;
        debug_info.verification_time_ms = verification_start.elapsed().as_millis() as u64;

        // 4. Extract model confidence
        let model_confidence = self.extract_confidence(output);
        debug_info.model_confidence = model_confidence;

        // 5. Run bias detection
        let bias_start = Instant::now();
        let bias_score = self.simulate_bias_detection(output).await;
        debug_info.bias_score = bias_score;
        debug_info.bias_detection_time_ms = bias_start.elapsed().as_millis() as u64;

        // 6. Arbiter decision
        let arbiter_start = Instant::now();
        let grounding = GroundingScore::new((grounding_score * 100.0) as usize, 100);
        let confidence = ConfidenceScore::new(model_confidence);
        let bias = BiasScore::new(bias_score, bias_score * 0.8, bias_score * 0.6, bias_score * 0.4, bias_score * 0.5);
        let verification = VerificationResult {
            status: if verification_valid {
                VerificationStatus::Valid
            } else {
                VerificationStatus::Invalid
            },
            proof: None,
            reasoning: "Pipeline verification result".to_string(),
            confidence: grounding_score,
        };

        let decision = self.make_decision(
            grounding,
            verification,
            confidence,
            bias,
            criticality,
            model_id,
        )?;
        debug_info.arbiter_time_ms = arbiter_start.elapsed().as_millis() as u64;

        // 7. Apply safety envelopes
        // (In production, this would actually modify the output)

        let latency_ms = start.elapsed().as_millis() as u64;
        let timed_out = latency_ms > self.config.pipeline_timeout_ms;

        Ok(PipelineResult {
            decision,
            latency_ms,
            timed_out,
            debug_info,
        })
    }

    /// Count sentences in output (simple heuristic)
    fn count_sentences(&self, text: &str) -> usize {
        text.matches('.').count()
            + text.matches('?').count()
            + text.matches('!').count()
    }

    /// Simulate KGS lookup
    async fn simulate_kgs_lookup(&self, _output: &str) -> f64 {
        // In production, this would call the actual KGS
        // For now, return a simulated score based on output length
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        0.75
    }

    /// Simulate formal verification
    async fn simulate_verification(&self, _output: &str) -> bool {
        // In production, this would call the formal verifier
        tokio::time::sleep(tokio::time::Duration::from_millis(3)).await;
        true
    }

    /// Extract confidence from output
    fn extract_confidence(&self, output: &str) -> f64 {
        // In production, this would use confidence extraction from bias-detector
        // Heuristic: output length affects confidence
        (output.len() as f64).min(1000.0) / 1000.0 * 0.9 + 0.1
    }

    /// Simulate bias detection
    async fn simulate_bias_detection(&self, _output: &str) -> f64 {
        // In production, this would call the bias detector
        tokio::time::sleep(tokio::time::Duration::from_millis(4)).await;
        0.15
    }

    /// Make arbiter decision
    fn make_decision(
        &self,
        grounding: GroundingScore,
        verification: VerificationResult,
        confidence: ConfidenceScore,
        bias: BiasScore,
        criticality: Criticality,
        model_id: &str,
    ) -> GatewayResult<AhfDecision> {
        let signals = DecisionSignals {
            grounding_score: grounding.score(),
            verification_valid: verification.status == VerificationStatus::Valid,
            model_confidence: confidence.0,
            bias_score: bias.score,
            criticality_level: format!("{:?}", criticality),
        };

        // Apply decision rules based on policy
        let policy = &self.config.policy;

        // Rule 1: Verification failed
        if verification.status == VerificationStatus::Invalid {
            return Ok(AhfDecision::new(
                Decision::Reject,
                DecisionReason::VerificationFailed,
                "Verification contradicted claim".to_string(),
                signals,
                false,
            ));
        }

        // Rule 2: Get thresholds for this model
        let grounding_threshold = policy.get_grounding_threshold(model_id);

        if grounding.score() < grounding_threshold {
            return Ok(AhfDecision::new(
                Decision::Reject,
                DecisionReason::LowGroundingScore,
                format!("Grounding score {} below threshold {}", grounding.score(), grounding_threshold),
                signals,
                false,
            ));
        }

        // Rule 3: Model confidence
        let confidence_threshold = policy.get_confidence_threshold(model_id);
        if confidence.0 < confidence_threshold {
            return Ok(AhfDecision::new(
                Decision::Reject,
                DecisionReason::LowConfidence,
                format!("Confidence {} below threshold {}", confidence.0, confidence_threshold),
                signals,
                false,
            ));
        }

        // Rule 4: Bias score
        let bias_threshold = policy.get_bias_threshold(model_id);
        if bias.score > bias_threshold {
            return Ok(AhfDecision::new(
                Decision::Reject,
                DecisionReason::HighBias,
                format!("Bias score {} exceeds threshold {}", bias.score, bias_threshold),
                signals,
                false,
            ));
        }

        // Rule 5: Criticality-based escalation
        if criticality == Criticality::Critical ||
           (criticality == Criticality::High && grounding.score() < 1.0) {
            return Ok(AhfDecision::new(
                Decision::Escalate,
                DecisionReason::HighCriticality,
                "High criticality claim requires escalation".to_string(),
                signals,
                false,
            ));
        }

        // Rule 6: All checks passed
        Ok(AhfDecision::new(
            Decision::Accept,
            DecisionReason::AllChecksPassed,
            "All verification checks passed".to_string(),
            signals,
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_basic() {
        let config = AhfConfig::default();
        let pipeline = AhfPipeline::new(config);

        let result = pipeline
            .verify(
                "Paris is the capital of France.",
                "gpt-4",
                Criticality::Medium,
            )
            .await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.latency_ms < 100);
    }

    #[tokio::test]
    async fn test_pipeline_empty_output() {
        let config = AhfConfig::default();
        let pipeline = AhfPipeline::new(config);

        let result = pipeline
            .verify("", "gpt-4", Criticality::Medium)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_debug_info() {
        let config = AhfConfig::default();
        let pipeline = AhfPipeline::new(config);

        let result = pipeline
            .verify(
                "Paris is the capital of France. It is beautiful.",
                "gpt-4",
                Criticality::Medium,
            )
            .await
            .unwrap();

        assert!(result.debug_info.claims_count > 0);
        assert!(result.debug_info.kgs_time_ms > 0);
    }

    #[test]
    fn test_count_sentences() {
        let config = AhfConfig::default();
        let pipeline = AhfPipeline::new(config);

        assert_eq!(pipeline.count_sentences("Hello. World."), 2);
        assert_eq!(pipeline.count_sentences("Is this a question?"), 1);
        assert_eq!(pipeline.count_sentences("Amazing!"), 1);
    }

    #[test]
    fn test_extract_confidence() {
        let config = AhfConfig::default();
        let pipeline = AhfPipeline::new(config);

        let short = pipeline.extract_confidence("short");
        let long_str = "a".repeat(1000);
        let long = pipeline.extract_confidence(&long_str);

        assert!(short < long);
        assert!(long <= 1.0);
    }
}
