//! Membership inference to identify high-confidence training data

use crate::Result;
use serde::{Deserialize, Serialize};

/// Score from membership inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipScore {
    /// Text that was evaluated
    pub text: String,
    /// Estimated loss when model generates this text
    pub loss: f32,
    /// Probability assigned by model
    pub probability: f32,
    /// Membership confidence (0.0-1.0)
    pub confidence: f32,
    /// Likely in training data
    pub likely_in_training: bool,
}

impl MembershipScore {
    /// Create a new membership score
    pub fn new(text: String, loss: f32, probability: f32) -> Self {
        // Higher probability and lower loss -> higher confidence of membership
        let confidence = (probability / (loss + 0.1).max(1.0)).min(1.0);
        let likely_in_training = confidence > 0.7;

        Self {
            text,
            loss,
            probability,
            confidence,
            likely_in_training,
        }
    }
}

/// Configuration for membership inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipInferenceConfig {
    /// Loss threshold for considering something as in-training
    pub loss_threshold: f32,
    /// Probability threshold
    pub probability_threshold: f32,
    /// Maximum number of samples to evaluate
    pub max_samples: usize,
    /// Batch size for evaluation
    pub batch_size: usize,
}

impl Default for MembershipInferenceConfig {
    fn default() -> Self {
        Self {
            loss_threshold: 2.0,
            probability_threshold: 0.1,
            max_samples: 10000,
            batch_size: 32,
        }
    }
}

/// Performs membership inference attacks to identify training data
pub struct MembershipInference {
    config: MembershipInferenceConfig,
    scores: Vec<MembershipScore>,
}

impl MembershipInference {
    /// Create a new membership inference engine
    pub fn new(config: MembershipInferenceConfig) -> Self {
        Self {
            config,
            scores: Vec::new(),
        }
    }

    /// Evaluate a single text sample
    ///
    /// In production, this would compute loss using actual model
    pub async fn evaluate_sample(&mut self, text: &str) -> Result<MembershipScore> {
        if self.scores.len() >= self.config.max_samples {
            return Err(crate::KefError::ExtractionFailed(
                "maximum samples evaluated".to_string(),
            ));
        }

        // Placeholder: simulate loss calculation
        // In production: run text through model and measure loss
        let loss = self.simulate_loss(text);
        let probability = self.simulate_probability(text);

        let score = MembershipScore::new(text.to_string(), loss, probability);
        self.scores.push(score.clone());

        Ok(score)
    }

    /// Evaluate a batch of samples
    pub async fn evaluate_batch(&mut self, texts: &[&str]) -> Result<Vec<MembershipScore>> {
        let mut results = Vec::new();

        for text in texts {
            if self.scores.len() >= self.config.max_samples {
                break;
            }

            let score = self.evaluate_sample(text).await?;
            results.push(score);
        }

        Ok(results)
    }

    /// Get all high-confidence membership samples
    pub fn high_confidence_samples(&self) -> Vec<&MembershipScore> {
        self.scores
            .iter()
            .filter(|score| {
                score.confidence > 0.7 && score.loss < self.config.loss_threshold
            })
            .collect()
    }

    /// Get scores
    pub fn scores(&self) -> &[MembershipScore] {
        &self.scores
    }

    /// Simulate loss computation (placeholder)
    fn simulate_loss(&self, text: &str) -> f32 {
        // In production: actual model loss computation
        // For now: text length -> loss heuristic
        let len = text.len() as f32;
        (100.0 / (len + 10.0)).min(3.0)
    }

    /// Simulate probability computation (placeholder)
    fn simulate_probability(&self, text: &str) -> f32 {
        // In production: actual model probability computation
        // For now: length-based heuristic
        let len = text.len() as f32;
        (len / (len + 100.0)).min(0.99)
    }
}

impl Default for MembershipInference {
    fn default() -> Self {
        Self::new(MembershipInferenceConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_membership_score_creation() {
        let score = MembershipScore::new("test text".to_string(), 1.5, 0.5);
        assert!(score.confidence >= 0.0 && score.confidence <= 1.0);
        assert_eq!(score.text, "test text");
    }

    #[test]
    fn test_high_confidence_detection() {
        let score = MembershipScore::new("this is a longer piece of text to achieve high confidence".to_string(), 1.0, 0.8);
        assert!(score.likely_in_training);
    }

    #[tokio::test]
    async fn test_evaluate_sample() -> Result<()> {
        let mut inference = MembershipInference::default();
        let score = inference.evaluate_sample("test text").await?;
        assert!(score.confidence >= 0.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_batch() -> Result<()> {
        let mut inference = MembershipInference::default();
        let texts = vec!["sample 1", "sample 2", "sample 3"];
        let scores = inference.evaluate_batch(&texts).await?;
        assert_eq!(scores.len(), 3);
        Ok(())
    }
}
