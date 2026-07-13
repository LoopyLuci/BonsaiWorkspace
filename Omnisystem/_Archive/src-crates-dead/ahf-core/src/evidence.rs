//! Evidence and verification types for the Anti-Hallucination Framework
//!
//! Defines structures for representing verification proofs, bias scores,
//! and evidence used in decision making.

use serde::{Deserialize, Serialize};
use crate::VerificationStatus;

/// Result of verifying a claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Status of verification
    pub status: VerificationStatus,
    /// Supporting evidence
    pub proof: Option<VerificationProof>,
    /// Reasoning for the result
    pub reasoning: String,
    /// Confidence in this verification result
    pub confidence: f64,
}

impl VerificationResult {
    /// Check if verification was successful
    pub fn is_verified(&self) -> bool {
        self.status == VerificationStatus::Valid
    }

    /// Check if verification found contradictions
    pub fn is_contradicted(&self) -> bool {
        self.status == VerificationStatus::Invalid
    }
}

/// Proof of verification with cryptographic integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationProof {
    /// Hash of the verified content
    pub content_hash: String,
    /// Sources consulted
    pub sources: Vec<String>,
    /// Signature or verification token
    pub signature: Option<String>,
}

/// Bias score measuring reasoning bias
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BiasScore {
    /// Overall bias score (0.0 = no bias, 1.0 = maximum bias)
    pub score: f64,
    /// Detected biases
    pub biases: [f64; 5], // [confirmation, anchoring, availability, recency, framing]
}

impl BiasScore {
    /// Create a new bias score from components
    pub fn new(confirmation: f64, anchoring: f64, availability: f64, recency: f64, framing: f64) -> Self {
        let scores = [confirmation, anchoring, availability, recency, framing];
        let clamped: Vec<f64> = scores.iter().map(|s| s.clamp(0.0, 1.0)).collect();
        let score = clamped.iter().sum::<f64>() / 5.0;

        BiasScore {
            score,
            biases: [clamped[0], clamped[1], clamped[2], clamped[3], clamped[4]],
        }
    }

    /// No bias detected
    pub fn clean() -> Self {
        BiasScore {
            score: 0.0,
            biases: [0.0; 5],
        }
    }

    /// Maximum bias detected
    pub fn compromised() -> Self {
        BiasScore {
            score: 1.0,
            biases: [1.0; 5],
        }
    }
}

/// Evidence supporting a claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Source reference
    pub source: String,
    /// Supporting text
    pub text: String,
    /// Relevance score (0.0 to 1.0)
    pub relevance: f64,
}

/// Bias violation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasViolation {
    /// Type of bias detected
    pub bias_type: String,
    /// Severity (0.0 to 1.0)
    pub severity: f64,
    /// Description of violation
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VerificationStatus;

    #[test]
    fn test_verification_result_verified() {
        let result = VerificationResult {
            status: VerificationStatus::Valid,
            proof: None,
            reasoning: "claim matches sources".to_string(),
            confidence: 0.95,
        };
        assert!(result.is_verified());
        assert!(!result.is_contradicted());
    }

    #[test]
    fn test_bias_score_calculation() {
        let bias = BiasScore::new(0.5, 0.3, 0.2, 0.1, 0.4);
        assert!((bias.score - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bias_score_clamping() {
        let bias = BiasScore::new(1.5, -0.5, 0.5, 0.5, 0.5);
        assert_eq!(bias.biases[0], 1.0);
        assert_eq!(bias.biases[1], 0.0);
    }

    #[test]
    fn test_bias_score_clean() {
        let bias = BiasScore::clean();
        assert_eq!(bias.score, 0.0);
        assert!(bias.biases.iter().all(|&b| b == 0.0));
    }

    #[test]
    fn test_bias_score_compromised() {
        let bias = BiasScore::compromised();
        assert_eq!(bias.score, 1.0);
        assert!(bias.biases.iter().all(|&b| b == 1.0));
    }
}
