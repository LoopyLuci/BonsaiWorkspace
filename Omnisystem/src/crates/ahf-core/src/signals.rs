//! Signal aggregation for the Anti-Hallucination Framework
//!
//! Combines outputs from KGS, Verifier, Bias Detector, and ConfidenceExtractor
//! into unified input signals for the Arbiter.

use serde::{Deserialize, Serialize};
use crate::{GroundingScore, VerificationResult, BiasScore, ConfidenceScore};

/// Combined signals for decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhfSignals {
    /// Grounding signal from KGS
    pub grounding_score: GroundingScore,
    /// Verification signal from Verifier
    pub verification_result: VerificationResult,
    /// Bias signal from BiasDetector
    pub bias_score: BiasScore,
    /// Confidence signal from ConfidenceExtractor
    pub model_confidence: ConfidenceScore,
}

impl AhfSignals {
    /// Create new signals
    pub fn new(
        grounding_score: GroundingScore,
        verification_result: VerificationResult,
        bias_score: BiasScore,
        model_confidence: ConfidenceScore,
    ) -> Self {
        AhfSignals {
            grounding_score,
            verification_result,
            bias_score,
            model_confidence,
        }
    }

    /// Check if signals are within reasonable bounds
    pub fn validate(&self) -> bool {
        self.grounding_score.score() >= 0.0 && self.grounding_score.score() <= 1.0
            && self.bias_score.score >= 0.0
            && self.bias_score.score <= 1.0
            && self.model_confidence.0 >= 0.0
            && self.model_confidence.0 <= 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VerificationStatus;

    #[test]
    fn test_signals_validation() {
        let signals = AhfSignals::new(
            GroundingScore::new(3, 5),
            VerificationResult {
                status: VerificationStatus::Valid,
                proof: None,
                reasoning: "test".to_string(),
                confidence: 0.9,
            },
            BiasScore::clean(),
            ConfidenceScore::new(0.8),
        );

        assert!(signals.validate());
    }
}
