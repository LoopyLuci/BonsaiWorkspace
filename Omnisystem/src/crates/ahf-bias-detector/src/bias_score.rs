//! Bias score aggregation and computation
//!
//! Combines scores from pattern matching and optional ML-based bias classification
//! into a unified bias score with severity levels.

use crate::bias_patterns::{BiasLevel, BiasViolation};
use crate::classifier::BiasClassifierResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Aggregated bias score with detailed violation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasScoreResult {
    /// Unique identifier
    pub id: Uuid,
    /// Overall bias score (0.0-1.0)
    pub score: f64,
    /// Maximum severity level detected
    pub max_severity: BiasLevel,
    /// Pattern-based violations
    pub pattern_violations: Vec<BiasViolation>,
    /// Optional classifier-based insights
    pub classifier_result: Option<BiasClassifierResult>,
    /// Human-readable summary
    pub summary: String,
}

impl BiasScoreResult {
    /// Create a new bias score result from pattern violations
    pub fn from_patterns(violations: Vec<BiasViolation>) -> Self {
        let max_severity = violations
            .iter()
            .map(|v| v.severity)
            .max()
            .unwrap_or(BiasLevel::Low);

        let score = if violations.is_empty() {
            0.0
        } else {
            // Score increases with number and severity of violations
            let severity_sum: u8 = violations.iter().map(|v| v.severity.score()).sum();
            let max_possible = violations.len() as u8 * BiasLevel::Critical.score();
            (severity_sum as f64 / max_possible as f64).clamp(0.0, 1.0)
        };

        let summary = Self::generate_summary(&violations, max_severity);

        BiasScoreResult {
            id: Uuid::new_v4(),
            score,
            max_severity,
            pattern_violations: violations,
            classifier_result: None,
            summary,
        }
    }

    /// Create with both pattern and classifier results
    pub fn with_classifier(mut self, classifier_result: BiasClassifierResult) -> Self {
        let combined_score = Self::combine_scores(self.score, classifier_result.confidence);
        self.score = combined_score;
        self.classifier_result = Some(classifier_result);
        self.summary = Self::generate_summary(&self.pattern_violations, self.max_severity);
        self
    }

    /// Combine pattern score and classifier score
    /// Uses max operator: whichever is higher
    fn combine_scores(pattern_score: f64, classifier_confidence: f64) -> f64 {
        pattern_score.max(classifier_confidence).clamp(0.0, 1.0)
    }

    /// Generate human-readable summary
    fn generate_summary(violations: &[BiasViolation], max_severity: BiasLevel) -> String {
        if violations.is_empty() {
            return "No bias detected.".to_string();
        }

        let count = violations.len();
        let plural = if count == 1 { "" } else { "s" };

        format!(
            "Detected {} bias violation{} with {} severity. {}",
            count,
            plural,
            match max_severity {
                BiasLevel::Low => "low",
                BiasLevel::Medium => "medium",
                BiasLevel::High => "high",
                BiasLevel::Critical => "critical",
            },
            Self::top_categories(violations)
        )
    }

    /// Get top categories from violations
    fn top_categories(violations: &[BiasViolation]) -> String {
        let mut categories: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for v in violations {
            *categories.entry(v.pattern.category.clone()).or_insert(0) += 1;
        }

        let mut top: Vec<_> = categories.iter().collect();
        top.sort_by(|a, b| b.1.cmp(a.1));

        let top_cats: Vec<String> = top
            .iter()
            .take(2)
            .map(|(k, v)| format!("{} ({})", k, v))
            .collect();

        format!("Top categories: {}", top_cats.join(", "))
    }

    /// Check if score is above threshold
    pub fn above_threshold(&self, threshold: f64) -> bool {
        self.score >= threshold
    }

    /// Check if bias is present (score > 0)
    pub fn has_bias(&self) -> bool {
        self.score > 0.0
    }

    /// Get critical violations only
    pub fn critical_violations(&self) -> Vec<&BiasViolation> {
        self.pattern_violations
            .iter()
            .filter(|v| v.severity == BiasLevel::Critical)
            .collect()
    }

    /// Get high or critical violations
    pub fn high_severity_violations(&self) -> Vec<&BiasViolation> {
        self.pattern_violations
            .iter()
            .filter(|v| v.severity >= BiasLevel::High)
            .collect()
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Bias score aggregator that combines multiple sources
pub struct BiasScoreAggregator {
    /// Enable pattern matching
    enable_patterns: bool,
    /// Enable classifier
    enable_classifier: bool,
    /// Minimum severity to include
    min_severity: BiasLevel,
}

impl BiasScoreAggregator {
    /// Create a new aggregator with both sources enabled
    pub fn new() -> Self {
        BiasScoreAggregator {
            enable_patterns: true,
            enable_classifier: true,
            min_severity: BiasLevel::Low,
        }
    }

    /// Enable or disable pattern matching
    pub fn with_patterns_enabled(mut self, enabled: bool) -> Self {
        self.enable_patterns = enabled;
        self
    }

    /// Enable or disable classifier
    pub fn with_classifier_enabled(mut self, enabled: bool) -> Self {
        self.enable_classifier = enabled;
        self
    }

    /// Set minimum severity to report
    pub fn with_min_severity(mut self, severity: BiasLevel) -> Self {
        self.min_severity = severity;
        self
    }

    /// Check if patterns are enabled
    pub fn patterns_enabled(&self) -> bool {
        self.enable_patterns
    }

    /// Check if classifier is enabled
    pub fn classifier_enabled(&self) -> bool {
        self.enable_classifier
    }

    /// Get minimum severity
    pub fn min_severity(&self) -> BiasLevel {
        self.min_severity
    }

    /// Filter violations by minimum severity
    pub fn filter_by_severity(&self, violations: Vec<BiasViolation>) -> Vec<BiasViolation> {
        violations
            .into_iter()
            .filter(|v| v.severity >= self.min_severity)
            .collect()
    }
}

impl Default for BiasScoreAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bias_patterns::BiasPattern;

    fn create_test_violation(severity: BiasLevel) -> BiasViolation {
        let pattern = BiasPattern::new(
            "test",
            r"test",
            "Test pattern",
            severity,
            "test_category",
        )
        .expect("Failed to create pattern");
        BiasViolation::new(pattern, (0, 4), "test")
    }

    #[test]
    fn test_bias_score_result_no_violations() {
        let result = BiasScoreResult::from_patterns(vec![]);
        assert_eq!(result.score, 0.0);
        assert_eq!(result.max_severity, BiasLevel::Low);
        assert!(!result.has_bias());
    }

    #[test]
    fn test_bias_score_result_single_violation() {
        let violations = vec![create_test_violation(BiasLevel::High)];
        let result = BiasScoreResult::from_patterns(violations);
        assert!(result.score > 0.0);
        assert_eq!(result.max_severity, BiasLevel::High);
        assert!(result.has_bias());
    }

    #[test]
    fn test_bias_score_result_multiple_violations() {
        let violations = vec![
            create_test_violation(BiasLevel::Low),
            create_test_violation(BiasLevel::Medium),
            create_test_violation(BiasLevel::High),
        ];
        let result = BiasScoreResult::from_patterns(violations);
        assert!(result.score > 0.0);
        assert_eq!(result.max_severity, BiasLevel::High);
        assert_eq!(result.pattern_violations.len(), 3);
    }

    #[test]
    fn test_bias_score_result_critical_violations() {
        let violations = vec![
            create_test_violation(BiasLevel::Low),
            create_test_violation(BiasLevel::Critical),
        ];
        let result = BiasScoreResult::from_patterns(violations);
        assert_eq!(result.max_severity, BiasLevel::Critical);
        assert_eq!(result.critical_violations().len(), 1);
    }

    #[test]
    fn test_bias_score_result_above_threshold() {
        let violations = vec![create_test_violation(BiasLevel::High)];
        let result = BiasScoreResult::from_patterns(violations);
        assert!(result.above_threshold(0.0));
        assert!(result.above_threshold(result.score - 0.01));
        assert!(!result.above_threshold(result.score + 0.01));
    }

    #[test]
    fn test_bias_score_result_high_severity_violations() {
        let violations = vec![
            create_test_violation(BiasLevel::Low),
            create_test_violation(BiasLevel::Medium),
            create_test_violation(BiasLevel::High),
            create_test_violation(BiasLevel::Critical),
        ];
        let result = BiasScoreResult::from_patterns(violations);
        assert_eq!(result.high_severity_violations().len(), 2); // High and Critical
    }

    #[test]
    fn test_bias_score_result_summary() {
        let violations = vec![
            create_test_violation(BiasLevel::High),
            create_test_violation(BiasLevel::Medium),
        ];
        let result = BiasScoreResult::from_patterns(violations);
        assert!(result.summary.contains("2"));
        assert!(result.summary.contains("high"));
    }

    #[test]
    fn test_bias_score_result_summary_no_violations() {
        let result = BiasScoreResult::from_patterns(vec![]);
        assert!(result.summary.contains("No bias"));
    }

    #[test]
    fn test_combine_scores_max() {
        let combined = BiasScoreResult::combine_scores(0.6, 0.8);
        assert_eq!(combined, 0.8);

        let combined = BiasScoreResult::combine_scores(0.9, 0.4);
        assert_eq!(combined, 0.9);
    }

    #[test]
    fn test_bias_score_result_to_json() {
        let violations = vec![create_test_violation(BiasLevel::High)];
        let result = BiasScoreResult::from_patterns(violations);
        let json = result.to_json().expect("Serialization failed");
        assert!(json.contains("score"));
    }

    #[test]
    fn test_bias_score_aggregator_defaults() {
        let agg = BiasScoreAggregator::new();
        assert!(agg.patterns_enabled());
        assert!(agg.classifier_enabled());
        assert_eq!(agg.min_severity(), BiasLevel::Low);
    }

    #[test]
    fn test_bias_score_aggregator_builder() {
        let agg = BiasScoreAggregator::new()
            .with_patterns_enabled(false)
            .with_classifier_enabled(false)
            .with_min_severity(BiasLevel::High);
        assert!(!agg.patterns_enabled());
        assert!(!agg.classifier_enabled());
        assert_eq!(agg.min_severity(), BiasLevel::High);
    }

    #[test]
    fn test_filter_by_severity() {
        let agg = BiasScoreAggregator::new().with_min_severity(BiasLevel::Medium);
        let violations = vec![
            create_test_violation(BiasLevel::Low),
            create_test_violation(BiasLevel::Medium),
            create_test_violation(BiasLevel::High),
        ];
        let filtered = agg.filter_by_severity(violations);
        assert_eq!(filtered.len(), 2); // Medium and High
        assert!(filtered.iter().all(|v| v.severity >= BiasLevel::Medium));
    }

    #[test]
    fn test_bias_score_result_with_classifier() {
        let violations = vec![create_test_violation(BiasLevel::Low)];
        let result = BiasScoreResult::from_patterns(violations);
        let original_score = result.score;

        let classifier_result = BiasClassifierResult {
            is_biased: true,
            confidence: 0.95,
            category: Some("subtle_bias".to_string()),
            explanation: "Subtle bias detected".to_string(),
        };

        let result_with_classifier = result.with_classifier(classifier_result);
        assert!(result_with_classifier.classifier_result.is_some());
        assert!(result_with_classifier.score >= original_score);
    }

    #[test]
    fn test_bias_score_result_score_clamping() {
        let violations = vec![create_test_violation(BiasLevel::Critical)];
        let result = BiasScoreResult::from_patterns(violations);
        assert!(result.score >= 0.0 && result.score <= 1.0);
    }
}
