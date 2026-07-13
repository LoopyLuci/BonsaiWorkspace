//! Test Utilities and Helpers
//!
//! Common utilities for integration testing, including test data builders,
//! assertion helpers, and logging utilities.

use crate::mock_model::{HallucinationCategory, HallucinationOutput};
use crate::hallucination_suite::HallucinationTestCase;
use uuid::Uuid;

/// Builder for creating hallucination outputs
pub struct HallucinationOutputBuilder {
    text: String,
    confidence: f64,
    category: HallucinationCategory,
    should_be_rejected: bool,
    description: String,
    domain: String,
}

impl HallucinationOutputBuilder {
    /// Create a new builder with defaults
    pub fn new(text: String) -> Self {
        Self {
            text,
            confidence: 0.8,
            category: HallucinationCategory::Fabrication,
            should_be_rejected: true,
            description: String::new(),
            domain: "general".to_string(),
        }
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set category
    pub fn with_category(mut self, category: HallucinationCategory) -> Self {
        self.category = category;
        self
    }

    /// Set rejection expectation
    pub fn should_reject(mut self, reject: bool) -> Self {
        self.should_be_rejected = reject;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// Set domain
    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = domain;
        self
    }

    /// Build the output
    pub fn build(self) -> HallucinationOutput {
        HallucinationOutput {
            id: Uuid::new_v4(),
            text: self.text,
            confidence: self.confidence,
            category: self.category,
            should_be_rejected: self.should_be_rejected,
            description: self.description,
            domain: self.domain,
        }
    }
}

/// Builder for creating test cases
pub struct TestCaseBuilder {
    prompt: String,
    expected_output: String,
    category: HallucinationCategory,
    should_reject: bool,
    domain: String,
    claimed_confidence: f64,
    rationale: String,
    tags: Vec<String>,
}

impl TestCaseBuilder {
    /// Create a new builder
    pub fn new(prompt: String, expected_output: String) -> Self {
        Self {
            prompt,
            expected_output,
            category: HallucinationCategory::Fabrication,
            should_reject: true,
            domain: "general".to_string(),
            claimed_confidence: 0.8,
            rationale: String::new(),
            tags: vec![],
        }
    }

    /// Set category
    pub fn with_category(mut self, category: HallucinationCategory) -> Self {
        self.category = category;
        self
    }

    /// Set rejection expectation
    pub fn should_reject(mut self, reject: bool) -> Self {
        self.should_reject = reject;
        self
    }

    /// Set domain
    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = domain;
        self
    }

    /// Set confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.claimed_confidence = confidence;
        self
    }

    /// Set rationale
    pub fn with_rationale(mut self, rationale: String) -> Self {
        self.rationale = rationale;
        self
    }

    /// Add tag
    pub fn add_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Set description (alias for with_rationale)
    pub fn with_description(mut self, description: String) -> Self {
        self.rationale = description;
        self
    }

    /// Build the test case
    pub fn build(self) -> HallucinationTestCase {
        HallucinationTestCase {
            id: Uuid::new_v4(),
            prompt: self.prompt,
            expected_output: self.expected_output,
            category: self.category,
            should_reject: self.should_reject,
            domain: self.domain,
            claimed_confidence: self.claimed_confidence,
            rationale: self.rationale,
            tags: self.tags,
        }
    }
}

/// Assertion helpers
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that accuracy is above threshold
    pub fn assert_accuracy_above(actual: f64, threshold: f64, message: &str) {
        assert!(
            actual >= threshold,
            "{}: Expected accuracy >= {:.2}%, got {:.2}%",
            message,
            threshold * 100.0,
            actual * 100.0
        );
    }

    /// Assert that false rejection rate is below threshold
    pub fn assert_false_rejection_below(actual: f64, threshold: f64, message: &str) {
        assert!(
            actual <= threshold,
            "{}: Expected false rejection <= {:.2}%, got {:.2}%",
            message,
            threshold * 100.0,
            actual * 100.0
        );
    }

    /// Assert that false negative rate is below threshold
    pub fn assert_false_negative_below(actual: f64, threshold: f64, message: &str) {
        assert!(
            actual <= threshold,
            "{}: Expected false negative <= {:.2}%, got {:.2}%",
            message,
            threshold * 100.0,
            actual * 100.0
        );
    }

    /// Assert that bias blocking rate is above threshold
    pub fn assert_bias_blocking_above(actual: f64, threshold: f64, message: &str) {
        assert!(
            actual >= threshold,
            "{}: Expected bias blocking >= {:.2}%, got {:.2}%",
            message,
            threshold * 100.0,
            actual * 100.0
        );
    }

    /// Assert that recovery time is within bounds
    pub fn assert_recovery_time_within(
        actual_ms: u64,
        max_ms: u64,
        message: &str,
    ) {
        assert!(
            actual_ms <= max_ms,
            "{}: Expected recovery <= {}ms, got {}ms",
            message,
            max_ms,
            actual_ms
        );
    }
}

/// Test statistics calculator
pub struct TestStats;

impl TestStats {
    /// Calculate mean of values
    pub fn mean(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        values.iter().sum::<f64>() / values.len() as f64
    }

    /// Calculate standard deviation
    pub fn stddev(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mean = Self::mean(values);
        let variance =
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        variance.sqrt()
    }

    /// Calculate percentile
    pub fn percentile(values: &[f64], p: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let index = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hallucination_output_builder() {
        let output = HallucinationOutputBuilder::new("Test hallucination".to_string())
            .with_confidence(0.95)
            .with_category(HallucinationCategory::Contradiction)
            .with_domain("geographic".to_string())
            .build();

        assert_eq!(output.text, "Test hallucination");
        assert_eq!(output.confidence, 0.95);
        assert_eq!(output.category, HallucinationCategory::Contradiction);
    }

    #[test]
    fn test_test_case_builder() {
        let case = TestCaseBuilder::new(
            "What is the capital of Atlantis?".to_string(),
            "Atlantis does not exist".to_string(),
        )
        .with_domain("geographic".to_string())
        .with_rationale("Fabricated place".to_string())
        .add_tag("fabrication".to_string())
        .build();

        assert_eq!(case.prompt, "What is the capital of Atlantis?");
        assert_eq!(case.domain, "geographic");
        assert!(case.tags.contains(&"fabrication".to_string()));
    }

    #[test]
    fn test_accuracy_assertion() {
        TestAssertions::assert_accuracy_above(0.95, 0.90, "Test");
    }

    #[test]
    #[should_panic]
    fn test_accuracy_assertion_fail() {
        TestAssertions::assert_accuracy_above(0.85, 0.90, "Test");
    }

    #[test]
    fn test_mean_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(TestStats::mean(&values), 3.0);
    }

    #[test]
    fn test_percentile_calculation() {
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let p50 = TestStats::percentile(&values, 50.0);
        assert_eq!(p50, 30.0);
    }
}
