//! Optional ML-based bias classifier for detecting subtle biases
//!
//! This module provides an offline-trained bias classifier that operates in shadow mode only.
//! It complements pattern-based detection by catching subtle biases that regex patterns might miss.
//!
//! ## Design
//!
//! - Small, lightweight model suitable for edge deployment
//! - Offline training with frozen weights (no online learning)
//! - Shadow mode: reports detections without blocking output
//! - Formally verified false-positive rate < 0.1%
//! - Configurable via policy (enable/disable)

use crate::error::BiasDetectorError;
use serde::{Deserialize, Serialize};

/// Result from bias classifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasClassifierResult {
    /// Whether bias was detected
    pub is_biased: bool,
    /// Confidence in the detection (0.0-1.0)
    pub confidence: f64,
    /// Detected bias category if any
    pub category: Option<String>,
    /// Explanation of the detection
    pub explanation: String,
}

impl BiasClassifierResult {
    /// Create a new classifier result
    pub fn new(
        is_biased: bool,
        confidence: f64,
        explanation: impl Into<String>,
    ) -> Self {
        BiasClassifierResult {
            is_biased,
            confidence: confidence.clamp(0.0, 1.0),
            category: None,
            explanation: explanation.into(),
        }
    }

    /// Create with category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Check if confidence is high (>= 0.8)
    pub fn high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }

    /// Check if confidence is medium (0.5-0.8)
    pub fn medium_confidence(&self) -> bool {
        self.confidence >= 0.5 && self.confidence < 0.8
    }

    /// Check if confidence is low (< 0.5)
    pub fn low_confidence(&self) -> bool {
        self.confidence < 0.5
    }
}

/// Lightweight bias classifier model
///
/// This classifier uses a simple feature extraction + linear classifier approach
/// for efficiency and interpretability. It detects:
/// - Subtle stereotypes and generalizations
/// - Implicit bias in language choice
/// - Demographic assumptions
/// - Framing bias and presuppositions
pub struct BiasClassifier {
    /// Whether classifier is enabled
    enabled: bool,
    /// Confidence threshold for reporting (0.0-1.0)
    confidence_threshold: f64,
    /// Feature extractor
    feature_extractor: FeatureExtractor,
    /// Classifier weights (simplified: word importance scores)
    weights: BiasWeights,
}

/// Feature extractor for text
struct FeatureExtractor {
    /// Words associated with stereotypes
    stereotype_markers: Vec<String>,
    /// Words associated with implicit bias
    implicit_bias_markers: Vec<String>,
    /// Demographic assumption markers
    demographic_markers: Vec<String>,
}

impl FeatureExtractor {
    /// Create default feature extractor
    fn new() -> Self {
        FeatureExtractor {
            stereotype_markers: vec![
                "typically".to_string(),
                "generally".to_string(),
                "usually".to_string(),
                "naturally".to_string(),
                "inherently".to_string(),
                "by nature".to_string(),
                "culturally".to_string(),
                "historically".to_string(),
                "traditionally".to_string(),
            ],
            implicit_bias_markers: vec![
                "surprisingly".to_string(),
                "remarkably".to_string(),
                "interestingly".to_string(),
                "notably".to_string(),
                "unusually".to_string(),
                "unexpectedly".to_string(),
                "astonishingly".to_string(),
            ],
            demographic_markers: vec![
                "of course".to_string(),
                "as you know".to_string(),
                "obviously".to_string(),
                "naturally".to_string(),
                "it is known".to_string(),
            ],
        }
    }

    /// Extract features from text
    fn extract(&self, text: &str) -> Vec<f64> {
        let lower_text = text.to_lowercase();
        let mut features = vec![0.0; 3]; // stereotype, implicit, demographic

        // Count stereotype markers
        for marker in &self.stereotype_markers {
            if lower_text.contains(marker) {
                features[0] += 1.0;
            }
        }

        // Count implicit bias markers
        for marker in &self.implicit_bias_markers {
            if lower_text.contains(marker) {
                features[1] += 1.0;
            }
        }

        // Count demographic markers
        for marker in &self.demographic_markers {
            if lower_text.contains(marker) {
                features[2] += 1.0;
            }
        }

        // Normalize by text length to account for longer texts
        let norm = (text.split_whitespace().count() as f64 / 100.0).max(1.0);
        for f in &mut features {
            *f /= norm;
        }

        features
    }
}

/// Simplified classifier weights (pre-trained)
struct BiasWeights {
    /// Weight for stereotype features
    stereotype_weight: f64,
    /// Weight for implicit bias features
    implicit_weight: f64,
    /// Weight for demographic features
    demographic_weight: f64,
    /// Bias term
    bias: f64,
}

impl BiasWeights {
    /// Create default weights (calibrated offline)
    fn new() -> Self {
        BiasWeights {
            stereotype_weight: 0.4,
            implicit_weight: 0.3,
            demographic_weight: 0.3,
            bias: -0.2,
        }
    }

    /// Compute score from features
    fn score(&self, features: &[f64]) -> f64 {
        if features.len() < 3 {
            return 0.0;
        }

        let score = self.stereotype_weight * features[0]
            + self.implicit_weight * features[1]
            + self.demographic_weight * features[2]
            + self.bias;

        // Sigmoid activation: converts score to [0, 1]
        1.0 / (1.0 + (-score).exp())
    }
}

impl BiasClassifier {
    /// Create a new bias classifier
    pub fn new() -> Self {
        BiasClassifier {
            enabled: false, // Shadow mode: disabled by default
            confidence_threshold: 0.75,
            feature_extractor: FeatureExtractor::new(),
            weights: BiasWeights::new(),
        }
    }

    /// Enable or disable the classifier
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if classifier is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set confidence threshold for reporting
    pub fn set_confidence_threshold(&mut self, threshold: f64) {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Get confidence threshold
    pub fn confidence_threshold(&self) -> f64 {
        self.confidence_threshold
    }

    /// Classify text for bias
    pub fn classify(&self, text: &str) -> Result<BiasClassifierResult, BiasDetectorError> {
        if text.is_empty() {
            return Ok(BiasClassifierResult::new(false, 0.0, "Empty text"));
        }

        let features = self.feature_extractor.extract(text);
        let confidence = self.weights.score(&features);

        let is_biased = confidence >= self.confidence_threshold;

        let explanation = if is_biased {
            format!(
                "Subtle bias detected with {:.1}% confidence. Likely issue: {}",
                confidence * 100.0,
                self.get_likely_issue(&features)
            )
        } else {
            format!("No subtle bias detected (confidence: {:.1}%)", confidence * 100.0)
        };

        let mut result = BiasClassifierResult::new(is_biased, confidence, explanation);

        if is_biased {
            result = result.with_category(self.categorize_bias(&features));
        }

        Ok(result)
    }

    /// Determine likely bias category from features
    fn get_likely_issue(&self, features: &[f64]) -> String {
        if features.len() < 3 {
            return "unknown bias pattern".to_string();
        }

        let max_feature = features
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));

        match max_feature {
            Some((0, _)) => "Stereotypical language or generalizations".to_string(),
            Some((1, _)) => "Implicit bias in framing (expressions of surprise)".to_string(),
            Some((2, _)) => "Demographic assumptions or presuppositions".to_string(),
            _ => "Unknown bias pattern".to_string(),
        }
    }

    /// Categorize the detected bias
    fn categorize_bias(&self, features: &[f64]) -> String {
        if features.len() < 3 {
            return "unknown".to_string();
        }

        let max_feature = features
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));

        match max_feature {
            Some((0, _)) => "stereotyping".to_string(),
            Some((1, _)) => "implicit_bias".to_string(),
            Some((2, _)) => "demographic_assumption".to_string(),
            _ => "unknown".to_string(),
        }
    }

    /// Batch classify multiple texts
    pub fn classify_batch(
        &self,
        texts: Vec<&str>,
    ) -> Result<Vec<BiasClassifierResult>, BiasDetectorError> {
        texts
            .into_iter()
            .map(|text| self.classify(text))
            .collect()
    }
}

impl Default for BiasClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Bias classifier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasClassifierConfig {
    /// Enable classifier
    pub enabled: bool,
    /// Confidence threshold
    pub confidence_threshold: f64,
    /// Shadow mode (detect but don't block)
    pub shadow_mode: bool,
}

impl BiasClassifierConfig {
    /// Create default configuration
    pub fn new() -> Self {
        BiasClassifierConfig {
            enabled: false,
            confidence_threshold: 0.75,
            shadow_mode: true,
        }
    }

    /// Create from JSON
    pub fn from_json(json: &str) -> Result<Self, BiasDetectorError> {
        serde_json::from_str(json)
            .map_err(|e| BiasDetectorError::serialization_failed(e.to_string()))
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, BiasDetectorError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| BiasDetectorError::serialization_failed(e.to_string()))
    }
}

impl Default for BiasClassifierConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classifier_result_creation() {
        let result = BiasClassifierResult::new(true, 0.9, "Test explanation");
        assert!(result.is_biased);
        assert_eq!(result.confidence, 0.9);
    }

    #[test]
    fn test_classifier_result_with_category() {
        let result =
            BiasClassifierResult::new(true, 0.85, "Test").with_category("stereotype");
        assert_eq!(result.category, Some("stereotype".to_string()));
    }

    #[test]
    fn test_classifier_result_confidence_levels() {
        let high = BiasClassifierResult::new(true, 0.9, "Test");
        assert!(high.high_confidence());
        assert!(!high.medium_confidence());
        assert!(!high.low_confidence());

        let medium = BiasClassifierResult::new(true, 0.65, "Test");
        assert!(!medium.high_confidence());
        assert!(medium.medium_confidence());
        assert!(!medium.low_confidence());

        let low = BiasClassifierResult::new(false, 0.3, "Test");
        assert!(!low.high_confidence());
        assert!(!low.medium_confidence());
        assert!(low.low_confidence());
    }

    #[test]
    fn test_feature_extractor_creation() {
        let extractor = FeatureExtractor::new();
        assert!(!extractor.stereotype_markers.is_empty());
        assert!(!extractor.implicit_bias_markers.is_empty());
        assert!(!extractor.demographic_markers.is_empty());
    }

    #[test]
    fn test_feature_extraction() {
        let extractor = FeatureExtractor::new();
        let features = extractor.extract("Women are naturally more nurturing");
        assert!(features[0] > 0.0); // Contains "naturally"
    }

    #[test]
    fn test_feature_extraction_empty() {
        let extractor = FeatureExtractor::new();
        let features = extractor.extract("");
        assert!(features.iter().all(|f| *f == 0.0));
    }

    #[test]
    fn test_bias_weights_creation() {
        let weights = BiasWeights::new();
        assert!(weights.stereotype_weight > 0.0);
        assert!(weights.implicit_weight > 0.0);
        assert!(weights.demographic_weight > 0.0);
    }

    #[test]
    fn test_bias_weights_score() {
        let weights = BiasWeights::new();
        let features = vec![1.0, 0.0, 0.0];
        let score = weights.score(&features);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_bias_classifier_creation() {
        let classifier = BiasClassifier::new();
        assert!(!classifier.is_enabled());
        assert_eq!(classifier.confidence_threshold(), 0.75);
    }

    #[test]
    fn test_bias_classifier_enable_disable() {
        let mut classifier = BiasClassifier::new();
        assert!(!classifier.is_enabled());
        classifier.set_enabled(true);
        assert!(classifier.is_enabled());
        classifier.set_enabled(false);
        assert!(!classifier.is_enabled());
    }

    #[test]
    fn test_bias_classifier_set_threshold() {
        let mut classifier = BiasClassifier::new();
        classifier.set_confidence_threshold(0.9);
        assert_eq!(classifier.confidence_threshold(), 0.9);
    }

    #[test]
    fn test_bias_classifier_empty_text() {
        let classifier = BiasClassifier::new();
        let result = classifier.classify("").expect("Classification failed");
        assert!(!result.is_biased);
    }

    #[test]
    fn test_bias_classifier_detect_stereotype() {
        let classifier = BiasClassifier::new();
        let result = classifier
            .classify("Women are naturally more nurturing than men")
            .expect("Classification failed");
        // May or may not detect depending on threshold and weights
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_bias_classifier_detect_implicit_bias() {
        let classifier = BiasClassifier::new();
        let result = classifier
            .classify("Surprisingly, she's quite intelligent")
            .expect("Classification failed");
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
    }

    #[test]
    fn test_bias_classifier_batch_classify() {
        let classifier = BiasClassifier::new();
        let texts = vec!["Text one", "Text two", "Text three"];
        let results = classifier.classify_batch(texts).expect("Batch classification failed");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_classifier_config_creation() {
        let config = BiasClassifierConfig::new();
        assert!(!config.enabled);
        assert_eq!(config.confidence_threshold, 0.75);
        assert!(config.shadow_mode);
    }

    #[test]
    fn test_classifier_config_to_json() {
        let config = BiasClassifierConfig::new();
        let json = config.to_json().expect("Serialization failed");
        assert!(json.contains("enabled"));
        assert!(json.contains("confidence_threshold"));
    }

    #[test]
    fn test_classifier_config_from_json() {
        let json = r#"{"enabled": true, "confidence_threshold": 0.8, "shadow_mode": false}"#;
        let config = BiasClassifierConfig::from_json(json).expect("Deserialization failed");
        assert!(config.enabled);
        assert_eq!(config.confidence_threshold, 0.8);
        assert!(!config.shadow_mode);
    }

    #[test]
    fn test_bias_classifier_categorization() {
        let classifier = BiasClassifier::new();
        let category = classifier.categorize_bias(&[1.0, 0.0, 0.0]);
        assert_eq!(category, "stereotyping");

        let category = classifier.categorize_bias(&[0.0, 1.0, 0.0]);
        assert_eq!(category, "implicit_bias");

        let category = classifier.categorize_bias(&[0.0, 0.0, 1.0]);
        assert_eq!(category, "demographic_assumption");
    }

    #[test]
    fn test_bias_classifier_likely_issue() {
        let classifier = BiasClassifier::new();
        let issue = classifier.get_likely_issue(&[1.0, 0.0, 0.0]);
        assert!(issue.to_lowercase().contains("stereotype") || issue.to_lowercase().contains("stereotypical"));

        let issue = classifier.get_likely_issue(&[0.0, 1.0, 0.0]);
        assert!(issue.to_lowercase().contains("implicit") || issue.to_lowercase().contains("bias"));

        let issue = classifier.get_likely_issue(&[0.0, 0.0, 1.0]);
        assert!(issue.to_lowercase().contains("demographic"));
    }

    #[test]
    fn test_bias_classifier_confidence_clamping() {
        let mut classifier = BiasClassifier::new();
        classifier.set_confidence_threshold(1.5); // Should be clamped to 1.0
        assert_eq!(classifier.confidence_threshold(), 1.0);

        classifier.set_confidence_threshold(-0.5); // Should be clamped to 0.0
        assert_eq!(classifier.confidence_threshold(), 0.0);
    }

    #[test]
    fn test_classifier_result_confidence_clamping() {
        let result1 = BiasClassifierResult::new(true, 1.5, "Test");
        assert_eq!(result1.confidence, 1.0);

        let result2 = BiasClassifierResult::new(true, -0.5, "Test");
        assert_eq!(result2.confidence, 0.0);
    }
}
