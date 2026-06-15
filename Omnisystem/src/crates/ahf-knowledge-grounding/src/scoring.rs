//! Scoring and evidence aggregation
//!
//! Aggregates evidence from multiple sources and calculates grounding scores.
//! Handles contradiction detection and reliability weighting.

use ahf_core::{GroundingScore, VerificationResult, VerificationStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Individual evidence item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceItem {
    /// Hash of the evidence statement (hex encoded)
    pub statement_hash: String,
    /// Hash of the source (hex encoded)
    pub source_hash: String,
    /// Source name (human-readable)
    pub source_name: String,
    /// Reliability score (0.0 to 1.0)
    pub reliability_score: f64,
}

impl EvidenceItem {
    /// Create new evidence item
    pub fn new(statement: &str, source_name: &str, reliability: f64) -> Self {
        let statement_hash = blake3::hash(statement.as_bytes()).to_hex().to_string();
        let source_hash = blake3::hash(source_name.as_bytes()).to_hex().to_string();

        EvidenceItem {
            statement_hash,
            source_hash,
            source_name: source_name.to_string(),
            reliability_score: reliability.clamp(0.0, 1.0),
        }
    }
}

/// Aggregated evidence set from multiple sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSet {
    /// Collection of evidence items
    pub items: Vec<EvidenceItem>,
    /// Aggregate reliability (0.0 to 1.0)
    pub aggregate_reliability: f64,
}

impl EvidenceSet {
    /// Create empty evidence set
    pub fn empty() -> Self {
        EvidenceSet {
            items: Vec::new(),
            aggregate_reliability: 0.0,
        }
    }

    /// Add evidence item
    pub fn add(&mut self, item: EvidenceItem) {
        self.items.push(item);
        self.recalculate_reliability();
    }

    /// Get count of evidence items
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Recalculate aggregate reliability
    fn recalculate_reliability(&mut self) {
        if self.items.is_empty() {
            self.aggregate_reliability = 0.0;
        } else {
            let sum: f64 = self.items.iter().map(|e| e.reliability_score).sum();
            self.aggregate_reliability = (sum / self.items.len() as f64).clamp(0.0, 1.0);
        }
    }

    /// Get unique sources in this evidence set
    pub fn unique_sources(&self) -> Vec<String> {
        let mut sources = HashMap::new();
        for item in &self.items {
            sources.insert(item.source_name.clone(), true);
        }
        sources.into_keys().collect()
    }
}

/// Grounding scorer for calculating verification scores
pub struct GroundingScorer {
    contradiction_threshold: f64,
}

impl GroundingScorer {
    /// Create a new scorer
    pub fn new() -> Self {
        GroundingScorer {
            contradiction_threshold: 0.5,
        }
    }

    /// Create with custom contradiction threshold
    pub fn with_threshold(threshold: f64) -> Self {
        GroundingScorer {
            contradiction_threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Calculate grounding score from verification results
    pub fn score_batch(&self, results: &[VerificationResult]) -> GroundingScore {
        if results.is_empty() {
            return GroundingScore::new(0, 0);
        }

        let mut verified = 0;
        let mut contradicted = 0;

        for result in results {
            if result.status == VerificationStatus::Invalid {
                contradicted += 1;
            } else if result.status == VerificationStatus::Valid {
                verified += 1;
            }
        }

        // If any contradiction found, score is 0
        if contradicted > 0 {
            return GroundingScore::contradicted();
        }

        GroundingScore::new(verified, results.len())
    }

    /// Calculate score from single verification result
    pub fn score_single(&self, result: &VerificationResult) -> GroundingScore {
        if result.status == VerificationStatus::Invalid {
            GroundingScore::contradicted()
        } else if result.status == VerificationStatus::Valid {
            GroundingScore::new(1, 1)
        } else {
            GroundingScore::new(0, 1)
        }
    }

    /// Aggregate scores from multiple verification results
    ///
    /// Returns (verified_count, total_count) tuple
    pub fn aggregate(&self, results: &[VerificationResult]) -> (usize, usize) {
        let mut verified = 0;
        let total = results.len();

        for result in results {
            if result.status == VerificationStatus::Valid {
                verified += 1;
            }
        }

        (verified, total)
    }
}

impl Default for GroundingScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_item_creation() {
        let item = EvidenceItem::new("test statement", "test source", 0.95);
        assert!(item.reliability_score > 0.94);
        assert_eq!(item.source_name, "test source");
    }

    #[test]
    fn test_evidence_set_add() {
        let mut set = EvidenceSet::empty();
        assert_eq!(set.count(), 0);

        set.add(EvidenceItem::new("test", "source1", 0.9));
        assert_eq!(set.count(), 1);
        assert!(set.aggregate_reliability > 0.8);
    }

    #[test]
    fn test_evidence_set_aggregate_reliability() {
        let mut set = EvidenceSet::empty();
        set.add(EvidenceItem::new("test1", "source1", 0.8));
        set.add(EvidenceItem::new("test2", "source2", 1.0));

        let expected = 0.9;
        assert!((set.aggregate_reliability - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_evidence_set_unique_sources() {
        let mut set = EvidenceSet::empty();
        set.add(EvidenceItem::new("test1", "source1", 0.9));
        set.add(EvidenceItem::new("test2", "source1", 0.9));
        set.add(EvidenceItem::new("test3", "source2", 0.9));

        let sources = set.unique_sources();
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn test_grounding_scorer_batch() {
        let scorer = GroundingScorer::new();

        let results = vec![
            VerificationResult {
                status: VerificationStatus::Verified,
                proof: None,
                reasoning: "verified".to_string(),
                confidence: 0.95,
            },
            VerificationResult {
                status: VerificationStatus::Verified,
                proof: None,
                reasoning: "verified".to_string(),
                confidence: 0.90,
            },
            VerificationResult {
                status: VerificationStatus::Unknown,
                proof: None,
                reasoning: "unknown".to_string(),
                confidence: 0.5,
            },
        ];

        let score = scorer.score_batch(&results);
        assert_eq!(score.verified, 2);
        assert_eq!(score.total, 3);
        assert!(score.score > 0.6);
    }

    #[test]
    fn test_grounding_scorer_contradiction() {
        let scorer = GroundingScorer::new();

        let results = vec![
            VerificationResult {
                status: VerificationStatus::Contradicted,
                proof: None,
                reasoning: "contradicted".to_string(),
                confidence: 0.95,
            },
        ];

        let score = scorer.score_batch(&results);
        assert_eq!(score.score, 0.0);
    }

    #[test]
    fn test_grounding_scorer_aggregate() {
        let scorer = GroundingScorer::new();

        let results = vec![
            VerificationResult {
                status: VerificationStatus::Verified,
                proof: None,
                reasoning: "verified".to_string(),
                confidence: 0.95,
            },
            VerificationResult {
                status: VerificationStatus::Unknown,
                proof: None,
                reasoning: "unknown".to_string(),
                confidence: 0.5,
            },
        ];

        let (verified, total) = scorer.aggregate(&results);
        assert_eq!(verified, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_grounding_scorer_single() {
        let scorer = GroundingScorer::new();

        let verified_result = VerificationResult {
            status: VerificationStatus::Valid,
            proof: None,
            reasoning: "verified".to_string(),
            confidence: 0.95,
        };

        let score = scorer.score_single(&verified_result);
        // GroundingScore doesn't expose these fields, just check the score is 1.0
        assert!(score.score() >= 0.9);
    }
}
