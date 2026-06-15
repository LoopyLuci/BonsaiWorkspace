//! Attention pattern extraction for knowledge triplet generation

use crate::Result;
use serde::{Deserialize, Serialize};

/// A knowledge triplet extracted from attention patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeTriplet {
    /// Subject entity
    pub subject: String,
    /// Relation/predicate
    pub relation: String,
    /// Object entity
    pub object: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Attention weights that support this triplet
    pub attention_weights: Vec<f32>,
}

/// Configuration for attention extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionExtractorConfig {
    /// Attention weight threshold
    pub weight_threshold: f32,
    /// Maximum number of triplets to extract
    pub max_triplets: usize,
    /// Attention head indices to analyze (empty = all)
    pub target_heads: Vec<usize>,
}

impl Default for AttentionExtractorConfig {
    fn default() -> Self {
        Self {
            weight_threshold: 0.5,
            max_triplets: 1000,
            target_heads: Vec::new(),
        }
    }
}

/// Extracts knowledge triplets from attention patterns
pub struct AttentionExtractor {
    config: AttentionExtractorConfig,
    triplets: Vec<KnowledgeTriplet>,
}

impl AttentionExtractor {
    /// Create a new attention extractor
    pub fn new(config: AttentionExtractorConfig) -> Self {
        Self {
            config,
            triplets: Vec::new(),
        }
    }

    /// Extract triplets from attention weights
    ///
    /// # Arguments
    ///
    /// * `tokens` - List of tokens in the sequence
    /// * `attention_weights` - 2D array of attention weights (seq_len x seq_len)
    ///
    /// # Errors
    ///
    /// Returns an error if extraction fails
    pub fn extract_from_attention(
        &mut self,
        tokens: &[String],
        attention_weights: &[Vec<f32>],
    ) -> Result<()> {
        if tokens.is_empty() || attention_weights.is_empty() {
            return Ok(());
        }

        // Find high-attention pairs
        for (i, weights) in attention_weights.iter().enumerate() {
            for (j, &weight) in weights.iter().enumerate() {
                if weight > self.config.weight_threshold && i != j && i < tokens.len() && j < tokens.len() {
                    // Create triplet: token[i] -> (via attention) -> token[j]
                    let triplet = self.tokens_to_triplet(&tokens[i], &tokens[j], weight);
                    self.add_triplet(triplet)?;
                }
            }
        }

        Ok(())
    }

    /// Add a triplet if not at capacity
    pub fn add_triplet(&mut self, triplet: KnowledgeTriplet) -> Result<()> {
        if self.triplets.len() < self.config.max_triplets {
            self.triplets.push(triplet);
        }
        Ok(())
    }

    /// Get extracted triplets
    pub fn triplets(&self) -> &[KnowledgeTriplet] {
        &self.triplets
    }

    /// Convert two tokens to a knowledge triplet
    fn tokens_to_triplet(
        &self,
        token_i: &str,
        token_j: &str,
        attention_weight: f32,
    ) -> KnowledgeTriplet {
        // Simple heuristic: subject (subject token) -> relates_to -> object (object token)
        KnowledgeTriplet {
            subject: token_i.to_string(),
            relation: "attends_to".to_string(),
            object: token_j.to_string(),
            confidence: attention_weight,
            attention_weights: vec![attention_weight],
        }
    }

    /// Validate triplet against known knowledge bases
    ///
    /// In production, this would query external KGs
    pub fn validate_triplet(&self, _triplet: &KnowledgeTriplet) -> bool {
        // Placeholder: all triplets are valid for now
        true
    }
}

impl Default for AttentionExtractor {
    fn default() -> Self {
        Self::new(AttentionExtractorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_creation() {
        let extractor = AttentionExtractor::default();
        assert_eq!(extractor.triplets().len(), 0);
    }

    #[test]
    fn test_triplet_creation() {
        let triplet = KnowledgeTriplet {
            subject: "cat".to_string(),
            relation: "is_a".to_string(),
            object: "animal".to_string(),
            confidence: 0.95,
            attention_weights: vec![0.95],
        };

        assert_eq!(triplet.subject, "cat");
        assert_eq!(triplet.object, "animal");
    }

    #[tokio::test]
    async fn test_extract_from_attention() -> Result<()> {
        let mut extractor = AttentionExtractor::default();
        let tokens = vec!["the".to_string(), "cat".to_string(), "sat".to_string()];

        let attention_weights = vec![
            vec![0.3, 0.5, 0.2],
            vec![0.2, 0.4, 0.4],
            vec![0.1, 0.6, 0.3],
        ];

        extractor.extract_from_attention(&tokens, &attention_weights)?;

        // Should have extracted some triplets
        assert!(extractor.triplets().len() > 0);

        Ok(())
    }
}
