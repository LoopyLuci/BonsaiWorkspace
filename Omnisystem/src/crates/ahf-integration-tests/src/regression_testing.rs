//! Regression Testing Framework
//!
//! Establishes baseline metrics and automatically detects regressions
//! in hallucination detection rates, false rejection rates, and bias handling.

use crate::test_metrics::TestMetrics;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Baseline metrics for regression detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionBaseline {
    /// Version of the AHF system
    pub version: String,
    /// Maximum acceptable hallucination detection rate below baseline
    pub hallucination_detection_rate: f64,
    /// Maximum acceptable false rejection rate increase
    pub false_rejection_rate: f64,
    /// Maximum acceptable false negative rate increase
    pub false_negative_rate: f64,
    /// Minimum acceptable bias blocking rate
    pub bias_blocking_rate: f64,
    /// Minimum acceptable calibration score
    pub calibration_score: f64,
    /// Baseline timestamp
    pub created_at: DateTime<Utc>,
    /// Per-category thresholds
    pub category_thresholds: HashMap<String, f64>,
    /// Per-domain thresholds
    pub domain_thresholds: HashMap<String, f64>,
    /// Notes about this baseline
    pub notes: String,
}

impl RegressionBaseline {
    /// Create default production baseline
    pub fn production() -> Self {
        let mut category_thresholds = HashMap::new();
        let mut domain_thresholds = HashMap::new();

        // Standard baselines for each category (detection rate should exceed these)
        category_thresholds.insert("Fabrication".to_string(), 0.95);
        category_thresholds.insert("Contradiction".to_string(), 0.98);
        category_thresholds.insert("TemporalViolation".to_string(), 0.92);
        category_thresholds.insert("Stereotype".to_string(), 0.90);
        category_thresholds.insert("SubtleBias".to_string(), 0.85);
        category_thresholds.insert("ConfidenceMismatch".to_string(), 0.88);
        category_thresholds.insert("FalseAttribution".to_string(), 0.93);
        category_thresholds.insert("NumericError".to_string(), 0.91);
        category_thresholds.insert("LogicalFallacy".to_string(), 0.87);
        category_thresholds.insert("ContextMisuse".to_string(), 0.86);

        // Standard baselines for each domain
        domain_thresholds.insert("geographic".to_string(), 0.97);
        domain_thresholds.insert("medical".to_string(), 0.95);
        domain_thresholds.insert("legal".to_string(), 0.96);
        domain_thresholds.insert("history".to_string(), 0.94);
        domain_thresholds.insert("science".to_string(), 0.93);
        domain_thresholds.insert("bias".to_string(), 0.88);

        Self {
            version: "1.0.0".to_string(),
            hallucination_detection_rate: 0.90, // Must detect 90%+ of hallucinations
            false_rejection_rate: 0.05,          // False rejections should be < 5%
            false_negative_rate: 0.10,           // Missing hallucinations should be < 10%
            bias_blocking_rate: 0.85,            // Bias detection should be 85%+
            calibration_score: 0.90,             // Confidence calibration should be good
            created_at: Utc::now(),
            category_thresholds,
            domain_thresholds,
            notes: "Production baseline established for Phase 5B validation suite".to_string(),
        }
    }

    /// Create permissive baseline for testing
    pub fn test() -> Self {
        Self {
            version: "1.0.0".to_string(),
            hallucination_detection_rate: 0.75,
            false_rejection_rate: 0.10,
            false_negative_rate: 0.25,
            bias_blocking_rate: 0.70,
            calibration_score: 0.70,
            created_at: Utc::now(),
            category_thresholds: HashMap::new(),
            domain_thresholds: HashMap::new(),
            notes: "Relaxed baseline for testing".to_string(),
        }
    }

    /// Check if metrics meet baseline
    pub fn check_metrics(&self, metrics: &TestMetrics) -> RegressionCheck {
        let mut regressions = Vec::new();
        let mut warnings = Vec::new();

        // Check overall hallucination detection rate
        if metrics.hallucination_detection_rate < self.hallucination_detection_rate {
            regressions.push(format!(
                "Hallucination detection rate {:.2}% below baseline {:.2}%",
                metrics.hallucination_detection_rate * 100.0,
                self.hallucination_detection_rate * 100.0
            ));
        }

        // Check false rejection rate
        if metrics.false_rejection_rate > self.false_rejection_rate {
            regressions.push(format!(
                "False rejection rate {:.2}% exceeds baseline {:.2}%",
                metrics.false_rejection_rate * 100.0,
                self.false_rejection_rate * 100.0
            ));
        }

        // Check false negative rate
        if metrics.false_negative_rate > self.false_negative_rate {
            warnings.push(format!(
                "False negative rate {:.2}% near baseline {:.2}%",
                metrics.false_negative_rate * 100.0,
                self.false_negative_rate * 100.0
            ));
        }

        // Check bias blocking rate
        if metrics.bias_blocking_rate < self.bias_blocking_rate {
            regressions.push(format!(
                "Bias blocking rate {:.2}% below baseline {:.2}%",
                metrics.bias_blocking_rate * 100.0,
                self.bias_blocking_rate * 100.0
            ));
        }

        // Check per-category thresholds
        for (category, threshold) in &self.category_thresholds {
            if let Some(cat_metrics) = metrics.results_by_category.get(category) {
                if cat_metrics.detection_rate < *threshold {
                    regressions.push(format!(
                        "Category {} detection rate {:.2}% below baseline {:.2}%",
                        category,
                        cat_metrics.detection_rate * 100.0,
                        threshold * 100.0
                    ));
                }
            }
        }

        // Check per-domain thresholds
        for (domain, threshold) in &self.domain_thresholds {
            if let Some(dom_metrics) = metrics.results_by_domain.get(domain) {
                if dom_metrics.detection_rate < *threshold {
                    regressions.push(format!(
                        "Domain {} detection rate {:.2}% below baseline {:.2}%",
                        domain,
                        dom_metrics.detection_rate * 100.0,
                        threshold * 100.0
                    ));
                }
            }
        }

        let passed = regressions.is_empty();

        RegressionCheck {
            baseline_version: self.version.clone(),
            passed,
            regressions,
            warnings,
            metrics_timestamp: Utc::now(),
        }
    }
}

impl Default for RegressionBaseline {
    fn default() -> Self {
        Self::production()
    }
}

/// Result of regression check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionCheck {
    /// AHF version checked
    pub baseline_version: String,
    /// Whether all checks passed
    pub passed: bool,
    /// List of detected regressions (failures)
    pub regressions: Vec<String>,
    /// List of warnings (near-baseline items)
    pub warnings: Vec<String>,
    /// When this check was run
    pub metrics_timestamp: DateTime<Utc>,
}

impl RegressionCheck {
    /// Get status summary
    pub fn status(&self) -> String {
        if self.passed {
            "PASS: All metrics within baseline".to_string()
        } else {
            format!("FAIL: {} regressions detected", self.regressions.len())
        }
    }

    /// Get detailed report
    pub fn report(&self) -> String {
        let mut report = format!("=== Regression Check Report ===\n");
        report.push_str(&format!("Baseline: {}\n", self.baseline_version));
        report.push_str(&format!("Status: {}\n\n", self.status()));

        if !self.regressions.is_empty() {
            report.push_str("REGRESSIONS:\n");
            for regression in &self.regressions {
                report.push_str(&format!("  - {}\n", regression));
            }
        }

        if !self.warnings.is_empty() {
            report.push_str("\nWARNINGS:\n");
            for warning in &self.warnings {
                report.push_str(&format!("  - {}\n", warning));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_baseline() {
        let baseline = RegressionBaseline::production();
        assert!(baseline.hallucination_detection_rate > 0.8);
        assert!(baseline.false_rejection_rate < 0.1);
    }

    #[test]
    fn test_baseline_check_pass() {
        let baseline = RegressionBaseline::test();
        let mut metrics = TestMetrics::new();
        metrics.hallucination_detection_rate = 0.85;
        metrics.false_rejection_rate = 0.05;
        metrics.bias_blocking_rate = 0.75;

        let check = baseline.check_metrics(&metrics);
        assert!(check.passed);
    }

    #[test]
    fn test_baseline_check_fail() {
        let baseline = RegressionBaseline::production();
        let mut metrics = TestMetrics::new();
        metrics.hallucination_detection_rate = 0.70; // Below baseline
        metrics.false_rejection_rate = 0.03;

        let check = baseline.check_metrics(&metrics);
        assert!(!check.passed);
        assert!(!check.regressions.is_empty());
    }

    #[test]
    fn test_regression_report() {
        let check = RegressionCheck {
            baseline_version: "1.0.0".to_string(),
            passed: false,
            regressions: vec!["Detection rate low".to_string()],
            warnings: vec!["False positive rate high".to_string()],
            metrics_timestamp: Utc::now(),
        };

        let report = check.report();
        assert!(report.contains("FAIL"));
        assert!(report.contains("Detection rate low"));
    }
}
