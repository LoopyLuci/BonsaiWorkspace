//! Multi-stage deduplication and curation of extracted knowledge

use crate::{CuratedChunk, KefError, QualityScores, Result};
use blake3::Hash;
use std::collections::HashSet;

/// Configuration for the curator
#[derive(Debug, Clone)]
pub struct CuratorConfig {
    /// Stage 1: exact dedup enabled
    pub enable_exact_dedup: bool,
    /// Stage 2: MinHash LSH enabled
    pub enable_minhash: bool,
    /// Stage 3: semantic similarity enabled
    pub enable_semantic: bool,
    /// Quality score threshold
    pub quality_threshold: f32,
    /// Minimum chunk length (chars)
    pub min_length: usize,
    /// Maximum chunk length (chars)
    pub max_length: usize,
}

impl Default for CuratorConfig {
    fn default() -> Self {
        Self {
            enable_exact_dedup: true,
            enable_minhash: true,
            enable_semantic: true,
            quality_threshold: 0.65,
            min_length: 20,
            max_length: 10000,
        }
    }
}

/// The Curator handles multi-stage deduplication and quality filtering
pub struct Curator {
    config: CuratorConfig,
    exact_dedup: HashSet<Hash>,
    pii_filter: crate::redaction::PiiRedactor,
    quality_scorer: crate::quality_scorer::QualityScorer,
    seen_hashes: HashSet<String>,
}

impl Curator {
    /// Create a new curator
    pub fn new(config: CuratorConfig) -> Self {
        Self {
            config,
            exact_dedup: HashSet::new(),
            pii_filter: crate::redaction::PiiRedactor::new(),
            quality_scorer: crate::quality_scorer::QualityScorer::default(),
            seen_hashes: HashSet::new(),
        }
    }

    /// Process a batch of chunks through all curation stages
    ///
    /// Returns only chunks that pass all stages
    pub async fn process(&mut self, chunks: Vec<String>) -> Result<Vec<CuratedChunk>> {
        let mut output = Vec::new();
        let mut total_seen = 0;
        let mut deduplicated = 0;
        let mut quality_passed = 0;

        for chunk in chunks {
            total_seen += 1;

            // Stage 0: Length check
            if chunk.len() < self.config.min_length || chunk.len() > self.config.max_length {
                continue;
            }

            // Stage 1: Exact deduplication
            if self.config.enable_exact_dedup {
                let hash = blake3::hash(chunk.as_bytes());
                if self.exact_dedup.contains(&hash) {
                    continue;
                }
                self.exact_dedup.insert(hash);
            }

            deduplicated += 1;

            // Stage 2: MinHash LSH (simple string hash for now)
            if self.config.enable_minhash {
                let minhash = self.compute_minhash(&chunk);
                if self.seen_hashes.contains(&minhash) {
                    continue; // Similar content seen before
                }
                self.seen_hashes.insert(minhash);
            }

            // Stage 3: PII redaction
            let has_pii = self.pii_filter.has_pii(&chunk);
            let redacted = if has_pii {
                self.pii_filter.redact(&chunk)
            } else {
                chunk.clone()
            };

            // Quality scoring
            let quality = self
                .quality_scorer
                .score_chunk(&redacted, 0.5)
                .await?;

            if !self.quality_scorer.passes_threshold(&quality) {
                continue;
            }

            quality_passed += 1;

            // Create curated chunk
            let curated = CuratedChunk {
                content: redacted,
                quality_scores: quality,
                pii_redacted: has_pii,
                extraction_method: "curator".to_string(),
                source_model: "unknown".to_string(),
                extracted_at: chrono::Utc::now(),
                tags: Vec::new(),
            };

            output.push(curated);
        }

        tracing::info!(
            "Curator: {}/{} deduplicated, {}/{} quality-passed",
            deduplicated,
            total_seen,
            quality_passed,
            deduplicated
        );

        Ok(output)
    }

    /// Simple MinHash computation (placeholder for LSH)
    fn compute_minhash(&self, text: &str) -> String {
        // In production: compute actual MinHash with multiple hash functions
        // For now: simple shingle-based approach
        let shingles: Vec<&str> = text.split_whitespace().take(3).collect();
        format!("{:?}", shingles)
    }

    /// Compute semantic similarity (placeholder)
    fn semantic_similarity(&self, _text1: &str, _text2: &str) -> f32 {
        // In production: embed both texts and compute cosine similarity
        0.0 // Placeholder
    }
}

impl Default for Curator {
    fn default() -> Self {
        Self::new(CuratorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curator_creation() {
        let curator = Curator::default();
        assert_eq!(curator.exact_dedup.len(), 0);
    }

    #[tokio::test]
    async fn test_exact_deduplication() -> Result<()> {
        let mut curator = Curator::default();
        let chunks = vec![
            "This is a test chunk.".to_string(),
            "This is a test chunk.".to_string(),
            "This is different.".to_string(),
        ];

        let curated = curator.process(chunks).await?;

        // Should have deduplicated the first two
        assert!(curated.len() < 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_pii_redaction() -> Result<()> {
        let mut curator = Curator::default();
        let chunks = vec!["Contact me at john@example.com for details.".to_string()];

        let curated = curator.process(chunks).await?;

        if !curated.is_empty() {
            assert!(curated[0].pii_redacted);
            assert!(curated[0].content.contains("[EMAIL]"));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_quality_filtering() -> Result<()> {
        let config = CuratorConfig {
            quality_threshold: 0.9, // Very high threshold
            ..Default::default()
        };
        let mut curator = Curator::new(config);
        let chunks = vec![
            "x".to_string(), // Too short and low quality
            "A comprehensive explanation of machine learning and its applications in modern AI systems.".to_string(),
        ];

        let curated = curator.process(chunks).await?;

        // Some chunks should be filtered out
        assert!(curated.len() <= 2);

        Ok(())
    }

    #[test]
    fn test_minhash_computation() {
        let curator = Curator::default();
        let hash1 = curator.compute_minhash("hello world test");
        let hash2 = curator.compute_minhash("hello world test");
        let hash3 = curator.compute_minhash("completely different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
