//! Test Metrics and Result Tracking
//!
//! Comprehensive metrics collection for hallucination detection accuracy,
//! false rejection rates, bias blocking, and other performance indicators.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test case ID
    pub test_id: String,
    /// Whether the test passed
    pub passed: bool,
    /// Expected decision (should_reject)
    pub expected: bool,
    /// Actual decision made by AHF
    pub actual: bool,
    /// Category of hallucination
    pub category: String,
    /// Domain of test
    pub domain: String,
    /// Reasoning for the decision
    pub reason: String,
    /// Timestamp of test
    pub timestamp: DateTime<Utc>,
}

/// Aggregated test metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMetrics {
    /// Total tests run
    pub total_tests: usize,
    /// Tests that passed
    pub passed_tests: usize,
    /// Tests that failed
    pub failed_tests: usize,
    /// Hallucination detection rate (true positive rate)
    pub hallucination_detection_rate: f64,
    /// False rejection rate (false positives)
    pub false_rejection_rate: f64,
    /// False negative rate (missed hallucinations)
    pub false_negative_rate: f64,
    /// Bias detection accuracy
    pub bias_blocking_rate: f64,
    /// Confidence calibration error
    pub calibration_error: f64,
    /// Average rejection confidence threshold
    pub avg_rejection_threshold: f64,
    /// Breakdown by category
    pub results_by_category: HashMap<String, CategoryMetrics>,
    /// Breakdown by domain
    pub results_by_domain: HashMap<String, DomainMetrics>,
    /// Individual test results
    pub test_results: Vec<TestResult>,
    /// Test execution time (milliseconds)
    pub execution_time_ms: u64,
    /// Timestamp of metrics
    pub timestamp: DateTime<Utc>,
}

/// Metrics broken down by hallucination category
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryMetrics {
    pub category: String,
    pub total: usize,
    pub detected: usize,
    pub missed: usize,
    pub false_positives: usize,
    pub detection_rate: f64,
}

/// Metrics broken down by domain
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainMetrics {
    pub domain: String,
    pub total: usize,
    pub detected: usize,
    pub missed: usize,
    pub false_positives: usize,
    pub detection_rate: f64,
}

impl TestMetrics {
    /// Create new metrics collection
    pub fn new() -> Self {
        Self {
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            hallucination_detection_rate: 0.0,
            false_rejection_rate: 0.0,
            false_negative_rate: 0.0,
            bias_blocking_rate: 0.0,
            calibration_error: 0.0,
            avg_rejection_threshold: 0.0,
            results_by_category: HashMap::new(),
            results_by_domain: HashMap::new(),
            test_results: Vec::new(),
            execution_time_ms: 0,
            timestamp: Utc::now(),
        }
    }

    /// Record a test result
    pub fn record_result(&mut self, result: TestResult) {
        self.total_tests += 1;
        if result.passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }
        self.test_results.push(result);
    }

    /// Finalize metrics (compute aggregations)
    pub fn finalize(&mut self) {
        // Compute overall rates
        if self.total_tests > 0 {
            self.hallucination_detection_rate = self.passed_tests as f64 / self.total_tests as f64;
        }

        // Count true positives, false positives, false negatives
        let mut true_positives = 0;
        let mut false_positives = 0;
        let mut false_negatives = 0;

        for result in &self.test_results {
            match (result.expected, result.actual) {
                (true, true) => true_positives += 1,   // Correctly rejected hallucination
                (true, false) => false_negatives += 1, // Missed hallucination
                (false, true) => false_positives += 1, // False rejection
                (false, false) => {},                  // Correct acceptance
            }
        }

        // Calculate rates
        let rejected_count = true_positives + false_positives;
        if rejected_count > 0 {
            self.false_rejection_rate = false_positives as f64 / rejected_count as f64;
        }

        let hallucination_count = true_positives + false_negatives;
        if hallucination_count > 0 {
            self.false_negative_rate = false_negatives as f64 / hallucination_count as f64;
        }

        // Bias detection: percentage of stereotype/bias tests that were caught
        let bias_tests: Vec<_> = self.test_results
            .iter()
            .filter(|r| r.category.contains("Bias") || r.category.contains("Stereotype"))
            .collect();
        if !bias_tests.is_empty() {
            let bias_detected = bias_tests.iter().filter(|r| r.actual).count();
            self.bias_blocking_rate = bias_detected as f64 / bias_tests.len() as f64;
        }

        // Compute category breakdowns
        self.compute_category_metrics();
        self.compute_domain_metrics();
    }

    fn compute_category_metrics(&mut self) {
        let mut category_stats: HashMap<String, (usize, usize, usize, usize)> = HashMap::new();

        for result in &self.test_results {
            let entry = category_stats
                .entry(result.category.clone())
                .or_insert((0, 0, 0, 0));

            entry.0 += 1; // total

            if result.passed {
                entry.1 += 1; // detected
            } else if result.expected {
                entry.2 += 1; // missed
            } else {
                entry.3 += 1; // false positives
            }
        }

        for (category, (total, detected, missed, fps)) in category_stats {
            let detection_rate = if total > 0 {
                detected as f64 / total as f64
            } else {
                0.0
            };

            self.results_by_category.insert(
                category.clone(),
                CategoryMetrics {
                    category,
                    total,
                    detected,
                    missed,
                    false_positives: fps,
                    detection_rate,
                },
            );
        }
    }

    fn compute_domain_metrics(&mut self) {
        let mut domain_stats: HashMap<String, (usize, usize, usize, usize)> = HashMap::new();

        for result in &self.test_results {
            let entry = domain_stats
                .entry(result.domain.clone())
                .or_insert((0, 0, 0, 0));

            entry.0 += 1; // total

            if result.passed {
                entry.1 += 1; // detected
            } else if result.expected {
                entry.2 += 1; // missed
            } else {
                entry.3 += 1; // false positives
            }
        }

        for (domain, (total, detected, missed, fps)) in domain_stats {
            let detection_rate = if total > 0 {
                detected as f64 / total as f64
            } else {
                0.0
            };

            self.results_by_domain.insert(
                domain.clone(),
                DomainMetrics {
                    domain,
                    total,
                    detected,
                    missed,
                    false_positives: fps,
                    detection_rate,
                },
            );
        }
    }

    /// Get a summary string of metrics
    pub fn summary(&self) -> String {
        format!(
            "TestMetrics {{\n  total: {}, passed: {}, failed: {},\n  hallucination_rate: {:.2}%,\n  false_rejection: {:.2}%,\n  false_negative: {:.2}%,\n  bias_blocking: {:.2}%\n}}",
            self.total_tests,
            self.passed_tests,
            self.failed_tests,
            self.hallucination_detection_rate * 100.0,
            self.false_rejection_rate * 100.0,
            self.false_negative_rate * 100.0,
            self.bias_blocking_rate * 100.0
        )
    }

    /// Check if metrics meet baseline requirements
    pub fn meets_baseline(&self) -> bool {
        self.hallucination_detection_rate > 0.90 && self.false_rejection_rate < 0.05
    }
}

impl Default for TestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = TestMetrics::new();
        assert_eq!(metrics.total_tests, 0);
        assert_eq!(metrics.passed_tests, 0);
    }

    #[test]
    fn test_record_result() {
        let mut metrics = TestMetrics::new();
        let result = TestResult {
            test_id: "test1".to_string(),
            passed: true,
            expected: true,
            actual: true,
            category: "Fabrication".to_string(),
            domain: "geographic".to_string(),
            reason: "Test passed".to_string(),
            timestamp: Utc::now(),
        };

        metrics.record_result(result);
        assert_eq!(metrics.total_tests, 1);
        assert_eq!(metrics.passed_tests, 1);
    }

    #[test]
    fn test_finalize_metrics() {
        let mut metrics = TestMetrics::new();

        for i in 0..10 {
            let result = TestResult {
                test_id: format!("test{}", i),
                passed: i < 8,
                expected: true,
                actual: i < 8,
                category: "Fabrication".to_string(),
                domain: "geographic".to_string(),
                reason: "Test".to_string(),
                timestamp: Utc::now(),
            };
            metrics.record_result(result);
        }

        metrics.finalize();
        assert_eq!(metrics.total_tests, 10);
        assert_eq!(metrics.passed_tests, 8);
        assert!(metrics.hallucination_detection_rate > 0.75);
    }

    #[test]
    fn test_summary() {
        let metrics = TestMetrics::new();
        let summary = metrics.summary();
        assert!(summary.contains("TestMetrics"));
    }
}
