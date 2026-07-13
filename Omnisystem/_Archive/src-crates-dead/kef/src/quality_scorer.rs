//! Quality scoring for extracted knowledge

use crate::{QualityScores, Result};

/// Configuration for quality scoring
#[derive(Debug, Clone)]
pub struct QualityScorerConfig {
    /// Minimum quality threshold (0.0-1.0)
    pub min_quality: f32,
    /// Weights for different quality dimensions
    pub relevance_weight: f32,
    pub accuracy_weight: f32,
    pub clarity_weight: f32,
    pub uniqueness_weight: f32,
}

impl Default for QualityScorerConfig {
    fn default() -> Self {
        Self {
            min_quality: 0.65,
            relevance_weight: 0.25,
            accuracy_weight: 0.35,
            clarity_weight: 0.25,
            uniqueness_weight: 0.15,
        }
    }
}

/// Scores the quality of extracted knowledge chunks
pub struct QualityScorer {
    config: QualityScorerConfig,
}

impl QualityScorer {
    /// Create a new quality scorer
    pub fn new(config: QualityScorerConfig) -> Self {
        Self { config }
    }

    /// Score a single chunk
    ///
    /// # Arguments
    ///
    /// * `content` - Text content to score
    /// * `uniqueness` - Uniqueness score (0.0-1.0)
    ///
    /// # Errors
    ///
    /// Returns an error if scoring fails
    pub async fn score_chunk(&self, content: &str, uniqueness: f32) -> Result<QualityScores> {
        // Compute individual quality dimensions
        let relevance = self.compute_relevance(content).await?;
        let accuracy = self.compute_accuracy(content).await?;
        let clarity = self.compute_clarity(content).await?;

        // Weighted aggregate
        let aggregate = (relevance * self.config.relevance_weight
            + accuracy * self.config.accuracy_weight
            + clarity * self.config.clarity_weight
            + uniqueness * self.config.uniqueness_weight)
            .max(0.0)
            .min(1.0);

        Ok(QualityScores {
            relevance,
            accuracy,
            clarity,
            uniqueness,
            aggregate,
        })
    }

    /// Score a batch of chunks
    pub async fn score_batch(&self, chunks: &[&str]) -> Result<Vec<QualityScores>> {
        let mut results = Vec::new();

        for chunk in chunks {
            let score = self.score_chunk(chunk, 0.5).await?;
            results.push(score);
        }

        Ok(results)
    }

    /// Compute relevance score
    async fn compute_relevance(&self, content: &str) -> Result<f32> {
        // Heuristics:
        // - Contains common NLP terms -> relevant
        // - Long enough (>50 chars) -> relevant
        // - Contains proper nouns -> relevant

        let mut score = 0.5; // baseline

        if content.len() > 50 {
            score += 0.2;
        }

        let nlp_terms = vec![
            "model", "learn", "train", "network", "algorithm", "data", "feature",
            "vector", "embedding", "attention", "knowledge", "reasoning",
        ];

        for term in nlp_terms {
            if content.to_lowercase().contains(term) {
                score += 0.05;
                break; // Only count once
            }
        }

        Ok(score.min(1.0))
    }

    /// Compute accuracy score
    async fn compute_accuracy(&self, content: &str) -> Result<f32> {
        // Heuristics:
        // - No obvious contradictions -> 0.8+
        // - Well-formed sentences -> 0.7+
        // - Contains citations/references -> 0.8+

        let mut score = 0.6; // baseline

        // Check sentence structure
        let sentences: Vec<&str> = content.split(|c| c == '.' || c == '!' || c == '?').collect();
        if sentences.len() > 2 {
            score += 0.15;
        }

        // Presence of qualifying language
        let qualifiers = vec!["research shows", "studies indicate", "evidence suggests"];
        for qualifier in qualifiers {
            if content.to_lowercase().contains(qualifier) {
                score += 0.1;
                break;
            }
        }

        Ok(score.min(1.0))
    }

    /// Compute clarity score
    async fn compute_clarity(&self, content: &str) -> Result<f32> {
        // Heuristics:
        // - No excessive punctuation/caps -> 0.7+
        // - Moderate length (100-500 chars) -> 0.8+
        // - Common vocabulary -> 0.7+

        let mut score = 0.5; // baseline

        let len = content.len();
        if len > 100 && len < 500 {
            score += 0.25;
        } else if len >= 500 {
            score += 0.15; // Very long texts might be harder to read
        }

        // Count excessive punctuation
        let punct_count = content.chars().filter(|c| "!?...".contains(*c)).count();
        if punct_count < (content.len() / 50).max(3) {
            score += 0.15;
        }

        // Check for ALL CAPS words (usually negates clarity)
        let all_caps_words = content
            .split_whitespace()
            .filter(|w| w.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()))
            .count();

        if all_caps_words == 0 {
            score += 0.1;
        }

        Ok(score.min(1.0))
    }

    /// Check if a score passes the quality threshold
    pub fn passes_threshold(&self, score: &QualityScores) -> bool {
        score.aggregate >= self.config.min_quality
    }
}

impl Default for QualityScorer {
    fn default() -> Self {
        Self::new(QualityScorerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_score_chunk() -> Result<()> {
        let scorer = QualityScorer::default();
        let content = "Machine learning is a subset of artificial intelligence that focuses on the development of algorithms and statistical models.";
        let score = scorer.score_chunk(content, 0.8).await?;

        assert!(score.aggregate >= 0.0);
        assert!(score.aggregate <= 1.0);
        assert!(score.relevance >= 0.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_relevance_scoring() -> Result<()> {
        let scorer = QualityScorer::default();
        let relevant = "This paper discusses neural network architectures and deep learning models.";
        let irrelevant = "The weather is nice today.";

        let rel_score = scorer.compute_relevance(relevant).await?;
        let irrel_score = scorer.compute_relevance(irrelevant).await?;

        assert!(rel_score > irrel_score);

        Ok(())
    }

    #[tokio::test]
    async fn test_clarity_scoring() -> Result<()> {
        let scorer = QualityScorer::default();
        let clear = "Machine learning algorithms learn patterns from data.";
        let unclear = "!!! BLAH??? xxx !!!";

        let clear_score = scorer.compute_clarity(clear).await?;
        let unclear_score = scorer.compute_clarity(unclear).await?;

        assert!(clear_score > unclear_score);

        Ok(())
    }

    #[tokio::test]
    async fn test_passes_threshold() -> Result<()> {
        let scorer = QualityScorer::default();
        let good_content = "Deep learning models have revolutionized artificial intelligence and machine learning.";
        let score = scorer.score_chunk(good_content, 0.9).await?;

        assert!(scorer.passes_threshold(&score));

        Ok(())
    }
}
