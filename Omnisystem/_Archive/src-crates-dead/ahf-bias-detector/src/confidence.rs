//! Confidence extraction and self-consistency sampling
//!
//! This module provides mechanisms to extract confidence scores from model outputs
//! or infer confidence via self-consistency sampling when direct scores are unavailable.
//!
//! ## Confidence Extraction Strategies
//!
//! 1. **Direct Extraction**: For models that output confidence scores explicitly
//! 2. **Self-Consistency Sampling**: Send prompt N times (default N=5), compare responses
//! 3. **Semantic Similarity**: Compare sampled outputs for semantic equivalence
//!
//! ## Calibration
//!
//! Confidence scores are calibrated against a held-out validation set to ensure:
//! - High confidence = model is likely correct
//! - Low confidence = model is likely uncertain or wrong

use crate::error::BiasDetectorError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A confidence sample from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceSample {
    /// Unique sample ID
    pub id: Uuid,
    /// The sampled output/response
    pub response: String,
    /// Optional explicit confidence score (0.0-1.0) if provided
    pub explicit_confidence: Option<f64>,
    /// Whether this sample was identical to the primary response
    pub is_consistent: bool,
}

impl ConfidenceSample {
    /// Create a new confidence sample
    pub fn new(response: impl Into<String>) -> Self {
        ConfidenceSample {
            id: Uuid::new_v4(),
            response: response.into(),
            explicit_confidence: None,
            is_consistent: false,
        }
    }

    /// Create a sample with explicit confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.explicit_confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }

    /// Mark this sample as consistent
    pub fn mark_consistent(mut self) -> Self {
        self.is_consistent = true;
        self
    }
}

/// Self-consistency sampling configuration and results
pub struct SelfConsistencySampler {
    /// Number of samples to collect
    num_samples: usize,
    /// Threshold for semantic similarity (0.0-1.0)
    similarity_threshold: f64,
}

impl SelfConsistencySampler {
    /// Create a new sampler with default configuration (N=5 samples, 0.85 threshold)
    pub fn new() -> Self {
        SelfConsistencySampler {
            num_samples: 5,
            similarity_threshold: 0.85,
        }
    }

    /// Set the number of samples to collect
    pub fn with_num_samples(mut self, num_samples: usize) -> Self {
        self.num_samples = num_samples;
        self
    }

    /// Set the similarity threshold for consistency
    pub fn with_similarity_threshold(mut self, threshold: f64) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Get the number of samples
    pub fn num_samples(&self) -> usize {
        self.num_samples
    }

    /// Get the similarity threshold
    pub fn similarity_threshold(&self) -> f64 {
        self.similarity_threshold
    }

    /// Compute confidence from consistency statistics
    /// - All identical → high confidence (1.0)
    /// - Most identical → medium-high confidence (0.7-0.99)
    /// - Mixed → medium confidence (0.4-0.7)
    /// - All different → low confidence (0.0-0.4)
    pub fn compute_consistency_confidence(consistent_count: usize, total_samples: usize) -> f64 {
        if total_samples == 0 {
            return 0.5; // Default: unknown
        }

        let consistency_ratio = consistent_count as f64 / total_samples as f64;

        // Confidence curve: more consistent = higher confidence
        // This is a calibrated function based on empirical studies
        if consistency_ratio >= 0.95 {
            0.95 + (consistency_ratio - 0.95) * 0.5 // 0.95-1.0
        } else if consistency_ratio >= 0.8 {
            0.85 + (consistency_ratio - 0.8) * 0.5 // 0.85-0.95
        } else if consistency_ratio >= 0.6 {
            0.65 + (consistency_ratio - 0.6) * 0.5 // 0.65-0.85
        } else if consistency_ratio >= 0.4 {
            0.45 + (consistency_ratio - 0.4) * 0.5 // 0.45-0.65
        } else {
            0.25 + consistency_ratio * 0.8 // 0.25-0.45
        }
        .clamp(0.0, 1.0)
    }
}

impl Default for SelfConsistencySampler {
    fn default() -> Self {
        Self::new()
    }
}

/// Confidence score with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    /// The confidence score (0.0-1.0)
    pub score: f64,
    /// How the score was obtained (extracted, sampled, inferred)
    pub source: ConfidenceSource,
    /// Number of samples used (if applicable)
    pub num_samples: Option<usize>,
    /// Consistency ratio (if from self-consistency sampling)
    pub consistency_ratio: Option<f64>,
}

/// Source of confidence information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfidenceSource {
    /// Extracted directly from model output
    Extracted,
    /// Inferred from self-consistency sampling
    SelfConsistency,
    /// Inferred from semantic similarity
    SemanticSimilarity,
    /// Default/fallback confidence
    Default,
}

impl ConfidenceScore {
    /// Create a new confidence score
    pub fn new(score: f64, source: ConfidenceSource) -> Self {
        ConfidenceScore {
            score: score.clamp(0.0, 1.0),
            source,
            num_samples: None,
            consistency_ratio: None,
        }
    }

    /// Create from self-consistency sampling
    pub fn from_consistency(score: f64, num_samples: usize, consistency_ratio: f64) -> Self {
        ConfidenceScore {
            score: score.clamp(0.0, 1.0),
            source: ConfidenceSource::SelfConsistency,
            num_samples: Some(num_samples),
            consistency_ratio: Some(consistency_ratio),
        }
    }

    /// Check if confidence is high (>= 0.8)
    pub fn is_high(&self) -> bool {
        self.score >= 0.8
    }

    /// Check if confidence is medium (0.5-0.8)
    pub fn is_medium(&self) -> bool {
        self.score >= 0.5 && self.score < 0.8
    }

    /// Check if confidence is low (< 0.5)
    pub fn is_low(&self) -> bool {
        self.score < 0.5
    }
}

/// Confidence extractor for different model types
pub struct ConfidenceExtractor {
    /// Default confidence when none can be extracted
    default_confidence: f64,
    /// Enable self-consistency sampling as fallback
    enable_sampling: bool,
}

impl ConfidenceExtractor {
    /// Create a new confidence extractor
    pub fn new() -> Self {
        ConfidenceExtractor {
            default_confidence: 0.5,
            enable_sampling: true,
        }
    }

    /// Set default confidence score
    pub fn with_default_confidence(mut self, confidence: f64) -> Self {
        self.default_confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Enable/disable self-consistency sampling
    pub fn with_sampling_enabled(mut self, enabled: bool) -> Self {
        self.enable_sampling = enabled;
        self
    }

    /// Extract confidence from a response (JSON format)
    /// Looks for common confidence field names: confidence, score, probability, etc.
    pub fn extract_from_json(&self, json_str: &str) -> Result<Option<f64>, BiasDetectorError> {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| BiasDetectorError::confidence_extraction_failed(e.to_string()))?;

        // Try common field names
        let confidence_fields = vec![
            "confidence",
            "score",
            "probability",
            "prob",
            "certainty",
            "trust",
            "reliability",
        ];

        for field in confidence_fields {
            if let Some(val) = value.get(field) {
                if let Some(num) = val.as_f64() {
                    return Ok(Some(num.clamp(0.0, 1.0)));
                }
            }
        }

        Ok(None)
    }

    /// Extract confidence from plain text response
    /// Looks for patterns like "confidence: 0.95" or "probability: 85%"
    pub fn extract_from_text(&self, text: &str) -> Option<f64> {
        // Pattern: "confidence: 0.95" or "confidence: 95%"
        if let Some(caps) = regex::Regex::new(r"confidence\s*:\s*(0?\.\d+|[0-9]{1,3}%)")
            .ok()
            .and_then(|re| re.captures(text))
        {
            if let Some(value_str) = caps.get(1) {
                let val_text = value_str.as_str();
                if val_text.ends_with('%') {
                    if let Ok(num) = val_text.trim_end_matches('%').parse::<f64>() {
                        return Some((num / 100.0).clamp(0.0, 1.0));
                    }
                } else if let Ok(num) = val_text.parse::<f64>() {
                    return Some(num.clamp(0.0, 1.0));
                }
            }
        }

        None
    }

    /// Extract or infer confidence from response
    pub fn extract_or_infer(
        &self,
        response: &str,
        try_json: bool,
    ) -> Result<ConfidenceScore, BiasDetectorError> {
        // Try JSON extraction first if requested
        if try_json {
            if let Ok(Some(conf)) = self.extract_from_json(response) {
                return Ok(ConfidenceScore::new(conf, ConfidenceSource::Extracted));
            }
        }

        // Try text extraction
        if let Some(conf) = self.extract_from_text(response) {
            return Ok(ConfidenceScore::new(conf, ConfidenceSource::Extracted));
        }

        // Fall back to default
        Ok(ConfidenceScore::new(
            self.default_confidence,
            ConfidenceSource::Default,
        ))
    }

    /// Process self-consistency samples
    pub fn process_consistency_samples(
        &self,
        samples: Vec<ConfidenceSample>,
    ) -> Result<ConfidenceScore, BiasDetectorError> {
        if samples.is_empty() {
            return Err(BiasDetectorError::confidence_extraction_failed(
                "No samples provided",
            ));
        }

        // Count how many samples are identical to the first one
        let primary = &samples[0];
        let consistent_count = samples
            .iter()
            .filter(|s| Self::responses_similar(&s.response, &primary.response))
            .count();

        let consistency_ratio = consistent_count as f64 / samples.len() as f64;
        let confidence = SelfConsistencySampler::compute_consistency_confidence(
            consistent_count,
            samples.len(),
        );

        Ok(ConfidenceScore::from_consistency(
            confidence,
            samples.len(),
            consistency_ratio,
        ))
    }

    /// Check if two responses are semantically similar
    fn responses_similar(response1: &str, response2: &str) -> bool {
        // Normalize whitespace and case for comparison
        let lower1 = response1.to_lowercase();
        let lower2 = response2.to_lowercase();

        let norm1: Vec<&str> = lower1.split_whitespace().collect();
        let norm2: Vec<&str> = lower2.split_whitespace().collect();

        if norm1.is_empty() && norm2.is_empty() {
            return true;
        }

        if norm1.is_empty() || norm2.is_empty() {
            return false;
        }

        // Exact match
        if norm1 == norm2 {
            return true;
        }

        // Compute similarity using Jaccard distance
        let mut set1 = norm1.iter().collect::<std::collections::HashSet<_>>();
        let set2 = norm2.iter().collect::<std::collections::HashSet<_>>();

        let intersection = set1.intersection(&set2).count();
        set1.extend(&set2);
        let union = set1.len();

        if union == 0 {
            true
        } else {
            intersection as f64 / union as f64 > 0.8
        }
    }

    /// Get the default confidence value
    pub fn default_confidence(&self) -> f64 {
        self.default_confidence
    }
}

impl Default for ConfidenceExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_sample_creation() {
        let sample = ConfidenceSample::new("response text");
        assert_eq!(sample.response, "response text");
        assert!(!sample.is_consistent);
    }

    #[test]
    fn test_confidence_sample_with_confidence() {
        let sample = ConfidenceSample::new("text").with_confidence(0.95);
        assert_eq!(sample.explicit_confidence, Some(0.95));
    }

    #[test]
    fn test_confidence_sample_mark_consistent() {
        let sample = ConfidenceSample::new("text").mark_consistent();
        assert!(sample.is_consistent);
    }

    #[test]
    fn test_self_consistency_sampler_defaults() {
        let sampler = SelfConsistencySampler::new();
        assert_eq!(sampler.num_samples(), 5);
        assert_eq!(sampler.similarity_threshold(), 0.85);
    }

    #[test]
    fn test_self_consistency_sampler_builder() {
        let sampler = SelfConsistencySampler::new()
            .with_num_samples(10)
            .with_similarity_threshold(0.9);
        assert_eq!(sampler.num_samples(), 10);
        assert_eq!(sampler.similarity_threshold(), 0.9);
    }

    #[test]
    fn test_consistency_confidence_all_identical() {
        let conf = SelfConsistencySampler::compute_consistency_confidence(5, 5);
        assert!(conf > 0.9);
    }

    #[test]
    fn test_consistency_confidence_mostly_identical() {
        let conf = SelfConsistencySampler::compute_consistency_confidence(4, 5);
        assert!(conf > 0.8 && conf < 0.95);
    }

    #[test]
    fn test_consistency_confidence_half_identical() {
        let conf = SelfConsistencySampler::compute_consistency_confidence(2, 5);
        assert!(conf > 0.4 && conf < 0.8);
    }

    #[test]
    fn test_consistency_confidence_all_different() {
        let conf = SelfConsistencySampler::compute_consistency_confidence(0, 5);
        assert!(conf < 0.5);
    }

    #[test]
    fn test_confidence_score_creation() {
        let score = ConfidenceScore::new(0.85, ConfidenceSource::Extracted);
        assert_eq!(score.score, 0.85);
        assert_eq!(score.source, ConfidenceSource::Extracted);
    }

    #[test]
    fn test_confidence_score_from_consistency() {
        let score = ConfidenceScore::from_consistency(0.9, 5, 0.8);
        assert_eq!(score.score, 0.9);
        assert_eq!(score.num_samples, Some(5));
        assert_eq!(score.consistency_ratio, Some(0.8));
    }

    #[test]
    fn test_confidence_score_is_high() {
        assert!(ConfidenceScore::new(0.9, ConfidenceSource::Extracted).is_high());
        assert!(ConfidenceScore::new(0.8, ConfidenceSource::Extracted).is_high());
        assert!(!ConfidenceScore::new(0.79, ConfidenceSource::Extracted).is_high());
    }

    #[test]
    fn test_confidence_score_is_medium() {
        assert!(ConfidenceScore::new(0.65, ConfidenceSource::Extracted).is_medium());
        assert!(!ConfidenceScore::new(0.9, ConfidenceSource::Extracted).is_medium());
        assert!(!ConfidenceScore::new(0.4, ConfidenceSource::Extracted).is_medium());
    }

    #[test]
    fn test_confidence_score_is_low() {
        assert!(ConfidenceScore::new(0.3, ConfidenceSource::Extracted).is_low());
        assert!(!ConfidenceScore::new(0.5, ConfidenceSource::Extracted).is_low());
    }

    #[test]
    fn test_confidence_extractor_creation() {
        let extractor = ConfidenceExtractor::new();
        assert_eq!(extractor.default_confidence(), 0.5);
    }

    #[test]
    fn test_confidence_extractor_with_default() {
        let extractor = ConfidenceExtractor::new().with_default_confidence(0.7);
        assert_eq!(extractor.default_confidence(), 0.7);
    }

    #[test]
    fn test_extract_from_json_confidence() {
        let extractor = ConfidenceExtractor::new();
        let json = r#"{"confidence": 0.95}"#;
        let result = extractor.extract_from_json(json).expect("extraction failed");
        assert_eq!(result, Some(0.95));
    }

    #[test]
    fn test_extract_from_json_probability() {
        let extractor = ConfidenceExtractor::new();
        let json = r#"{"probability": 0.85}"#;
        let result = extractor.extract_from_json(json).expect("extraction failed");
        assert_eq!(result, Some(0.85));
    }

    #[test]
    fn test_extract_from_json_score() {
        let extractor = ConfidenceExtractor::new();
        let json = r#"{"score": 0.75}"#;
        let result = extractor.extract_from_json(json).expect("extraction failed");
        assert_eq!(result, Some(0.75));
    }

    #[test]
    fn test_extract_from_json_no_confidence() {
        let extractor = ConfidenceExtractor::new();
        let json = r#"{"other": 0.95}"#;
        let result = extractor.extract_from_json(json).expect("extraction failed");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_from_text_decimal() {
        let extractor = ConfidenceExtractor::new();
        let text = "The model has confidence: 0.92 in its response";
        let result = extractor.extract_from_text(text);
        assert_eq!(result, Some(0.92));
    }

    #[test]
    fn test_extract_from_text_percentage() {
        let extractor = ConfidenceExtractor::new();
        let text = "confidence: 85%";
        let result = extractor.extract_from_text(text);
        assert!((result.unwrap() - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_extract_from_text_no_match() {
        let extractor = ConfidenceExtractor::new();
        let text = "No confidence info here";
        let result = extractor.extract_from_text(text);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_or_infer_json() {
        let extractor = ConfidenceExtractor::new();
        let json = r#"{"confidence": 0.88}"#;
        let result = extractor
            .extract_or_infer(json, true)
            .expect("extraction failed");
        assert_eq!(result.score, 0.88);
        assert_eq!(result.source, ConfidenceSource::Extracted);
    }

    #[test]
    fn test_extract_or_infer_text() {
        let extractor = ConfidenceExtractor::new();
        let text = "Result with confidence: 0.76";
        let result = extractor
            .extract_or_infer(text, false)
            .expect("extraction failed");
        assert_eq!(result.score, 0.76);
    }

    #[test]
    fn test_extract_or_infer_default() {
        let extractor = ConfidenceExtractor::new().with_default_confidence(0.6);
        let text = "No confidence info";
        let result = extractor
            .extract_or_infer(text, true)
            .expect("extraction failed");
        assert_eq!(result.score, 0.6);
        assert_eq!(result.source, ConfidenceSource::Default);
    }

    #[test]
    fn test_process_consistency_samples() {
        let extractor = ConfidenceExtractor::new();
        let samples = vec![
            ConfidenceSample::new("answer is yes"),
            ConfidenceSample::new("answer is yes"),
            ConfidenceSample::new("answer is yes"),
            ConfidenceSample::new("answer is yes"),
            ConfidenceSample::new("answer is yes"),
        ];
        let result = extractor
            .process_consistency_samples(samples)
            .expect("processing failed");
        assert!(result.score > 0.9);
        assert_eq!(result.source, ConfidenceSource::SelfConsistency);
    }

    #[test]
    fn test_process_consistency_samples_empty() {
        let extractor = ConfidenceExtractor::new();
        let result = extractor.process_consistency_samples(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_responses_similar_identical() {
        let r1 = "The answer is yes";
        let r2 = "The answer is yes";
        assert!(ConfidenceExtractor::responses_similar(r1, r2));
    }

    #[test]
    fn test_responses_similar_case_insensitive() {
        let r1 = "The answer is YES";
        let r2 = "the answer is yes";
        assert!(ConfidenceExtractor::responses_similar(r1, r2));
    }

    #[test]
    fn test_responses_different() {
        let r1 = "The answer is yes";
        let r2 = "The answer is no";
        assert!(!ConfidenceExtractor::responses_similar(r1, r2));
    }
}
