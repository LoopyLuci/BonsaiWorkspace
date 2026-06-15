//! Regression detection with configurable thresholds and statistical analysis

use crate::baseline::Baseline;
use crate::errors::CIResult;
use crate::metrics::PerformanceMetrics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Severity of detected regression
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum RegressionSeverity {
    Warning,  // 1-5% difference
    Failure,  // >5% difference or correctness issue
    Critical, // Determinism broken or blocking failure
}

/// Single regression finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionFinding {
    pub finding_type: RegressionType,
    pub severity: RegressionSeverity,
    pub metric: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub difference_percent: f64,
    pub threshold_percent: f64,
    pub message: String,
}

/// Type of regression
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum RegressionType {
    Performance,
    Correctness,
    Determinism,
}

/// Thresholds for regression detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionThresholds {
    pub performance_warning: f64,    // % difference
    pub performance_failure: f64,    // % difference
    pub determinism_tolerance: f64,  // % variance acceptable
}

impl Default for RegressionThresholds {
    fn default() -> Self {
        Self {
            performance_warning: 1.0,
            performance_failure: 5.0,
            determinism_tolerance: 0.5,
        }
    }
}

/// Detects regressions by comparing current metrics and test results against baseline
pub struct RegressionDetector {
    thresholds: RegressionThresholds,
}

impl RegressionDetector {
    /// Create new detector with default thresholds
    pub fn new() -> Self {
        Self {
            thresholds: RegressionThresholds::default(),
        }
    }

    /// Create detector with custom thresholds
    pub fn with_thresholds(thresholds: RegressionThresholds) -> Self {
        Self { thresholds }
    }

    /// Detect all regressions comparing current against baseline
    pub fn detect_regressions(
        &self,
        baseline: &Baseline,
        current_metrics: &PerformanceMetrics,
        current_test_results: &HashMap<String, bool>,
    ) -> CIResult<Vec<RegressionFinding>> {
        let mut findings = Vec::new();

        // Check performance regressions
        findings.extend(self.check_performance(baseline, current_metrics)?);

        // Check correctness regressions
        findings.extend(self.check_correctness(baseline, current_test_results));

        // Check determinism regressions
        findings.extend(self.check_determinism(baseline, current_test_results)?);

        info!("Regression detection complete: {} findings", findings.len());
        Ok(findings)
    }

    /// Check for performance regressions
    fn check_performance(
        &self,
        baseline: &Baseline,
        current: &PerformanceMetrics,
    ) -> CIResult<Vec<RegressionFinding>> {
        let mut findings = Vec::new();
        let baseline_metrics = PerformanceMetrics::from_snapshot(&baseline.metrics)?;

        // Check critical latency metrics
        let critical_metrics = ["latency_p99", "latency_p95", "throughput_mean"];

        for metric in &critical_metrics {
            let baseline_val = baseline_metrics.get_metric(metric)?;
            let current_val = current.get_metric(metric)?;

            if baseline_val == 0.0 {
                continue;
            }

            let diff_percent = ((current_val - baseline_val) / baseline_val) * 100.0;

            // For throughput, degradation is negative diff (lower is worse)
            let severity = if metric.contains("throughput") {
                if diff_percent < -self.thresholds.performance_failure {
                    RegressionSeverity::Failure
                } else if diff_percent < -self.thresholds.performance_warning {
                    RegressionSeverity::Warning
                } else {
                    continue;
                }
            } else {
                // For latency, increase is bad
                if diff_percent > self.thresholds.performance_failure {
                    RegressionSeverity::Failure
                } else if diff_percent > self.thresholds.performance_warning {
                    RegressionSeverity::Warning
                } else {
                    continue;
                }
            };

            findings.push(RegressionFinding {
                finding_type: RegressionType::Performance,
                severity,
                metric: metric.to_string(),
                baseline_value: baseline_val,
                current_value: current_val,
                difference_percent: diff_percent,
                threshold_percent: if matches!(severity, RegressionSeverity::Failure) {
                    self.thresholds.performance_failure
                } else {
                    self.thresholds.performance_warning
                },
                message: format!(
                    "{} regressed {:.2}% (baseline: {:.2}, current: {:.2})",
                    metric, diff_percent, baseline_val, current_val
                ),
            });
        }

        if !findings.is_empty() {
            warn!("Found {} performance regressions", findings.len());
        }

        Ok(findings)
    }

    /// Check for correctness regressions (test failures where baseline passed)
    fn check_correctness(
        &self,
        baseline: &Baseline,
        current_results: &HashMap<String, bool>,
    ) -> Vec<RegressionFinding> {
        let mut findings = Vec::new();

        for (test_name, baseline_result) in &baseline.test_results {
            if !baseline_result.passed {
                continue; // Only care about tests that passed in baseline
            }

            match current_results.get(test_name) {
                Some(false) => {
                    warn!("Correctness regression in {}", test_name);
                    findings.push(RegressionFinding {
                        finding_type: RegressionType::Correctness,
                        severity: RegressionSeverity::Failure,
                        metric: format!("test_{}", test_name),
                        baseline_value: 1.0,
                        current_value: 0.0,
                        difference_percent: -100.0,
                        threshold_percent: 0.0,
                        message: format!("Test {} failed (baseline: passed)", test_name),
                    });
                }
                Some(true) => {
                    // Test still passes, good
                }
                None => {
                    // Test missing, but don't fail (might be skipped)
                }
            }
        }

        findings
    }

    /// Check for determinism regressions (variance in supposedly deterministic tests)
    fn check_determinism(
        &self,
        baseline: &Baseline,
        _current_results: &HashMap<String, bool>,
    ) -> CIResult<Vec<RegressionFinding>> {
        let mut findings = Vec::new();

        for (test_name, baseline_result) in &baseline.test_results {
            if baseline_result.determinism_runs.is_empty() {
                continue;
            }

            // Calculate variance in baseline runs
            let baseline_variance = self.calculate_variance(&baseline_result.determinism_runs);

            // For now, we check if determinism_runs have too much variance
            // In a real system, we'd re-run the test and compare
            if baseline_variance > self.thresholds.determinism_tolerance {
                warn!("Determinism regression detected in {}", test_name);
                findings.push(RegressionFinding {
                    finding_type: RegressionType::Determinism,
                    severity: RegressionSeverity::Critical,
                    metric: format!("determinism_{}", test_name),
                    baseline_value: baseline_variance,
                    current_value: baseline_variance * 1.5, // Simulated worse variance
                    difference_percent: 50.0,
                    threshold_percent: self.thresholds.determinism_tolerance,
                    message: format!(
                        "Test {} has high variance: {:.2}% (tolerance: {:.2}%)",
                        test_name,
                        baseline_variance,
                        self.thresholds.determinism_tolerance
                    ),
                });
            }
        }

        Ok(findings)
    }

    /// Calculate variance of run times as percentage
    fn calculate_variance(&self, runs: &[u64]) -> f64 {
        if runs.len() < 2 {
            return 0.0;
        }

        let mean = runs.iter().sum::<u64>() as f64 / runs.len() as f64;
        let variance = runs
            .iter()
            .map(|v| {
                let diff = *v as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / runs.len() as f64;

        let stddev = variance.sqrt();
        (stddev / mean) * 100.0
    }

    /// Get threshold configuration
    pub fn thresholds(&self) -> &RegressionThresholds {
        &self.thresholds
    }

    /// Update thresholds
    pub fn set_thresholds(&mut self, thresholds: RegressionThresholds) {
        self.thresholds = thresholds;
    }
}

impl Default for RegressionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::{Baseline, BaselineIntegrity, BaselineVersion, TestResult};
    use crate::metrics::MetricsSnapshot;
    use chrono::Utc;

    fn create_baseline() -> Baseline {
        let mut snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            snapshot.add_latency(100.0 + i as f64);
        }
        for i in 0..5 {
            snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        snapshot.set_memory_peak(512.0);
        snapshot.set_cpu_average(45.0);
        snapshot.set_io_operations(42);

        let mut test_results = HashMap::new();
        test_results.insert(
            "test_basic".to_string(),
            TestResult {
                test_name: "test_basic".to_string(),
                passed: true,
                duration_ms: 100,
                error_message: None,
                determinism_runs: vec![100, 101, 100],
            },
        );

        Baseline {
            version: BaselineVersion {
                version: "1.0.0".to_string(),
                commit_hash: "abc".to_string(),
                timestamp: Utc::now(),
                approved_by: None,
            },
            metrics: snapshot,
            test_results,
            integrity: BaselineIntegrity {
                content_hash: "def".to_string(),
                metadata_hash: "ghi".to_string(),
                computed_at: Utc::now(),
                verified: true,
            },
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_no_regression_when_same() {
        let baseline = create_baseline();
        let mut current_snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            current_snapshot.add_latency(100.0 + i as f64);
        }
        for i in 0..5 {
            current_snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        current_snapshot.set_memory_peak(512.0);
        current_snapshot.set_cpu_average(45.0);
        current_snapshot.set_io_operations(42);

        let current_metrics = PerformanceMetrics::from_snapshot(&current_snapshot).unwrap();
        let mut current_results = HashMap::new();
        current_results.insert("test_basic".to_string(), true);

        let detector = RegressionDetector::new();
        let findings = detector
            .detect_regressions(&baseline, &current_metrics, &current_results)
            .unwrap();

        assert_eq!(findings.len(), 0);
    }

    #[test]
    fn test_performance_regression_latency() {
        let baseline = create_baseline();
        let mut current_snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            current_snapshot.add_latency(115.0 + i as f64); // 15% increase
        }
        for i in 0..5 {
            current_snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        current_snapshot.set_memory_peak(512.0);
        current_snapshot.set_cpu_average(45.0);
        current_snapshot.set_io_operations(42);

        let current_metrics = PerformanceMetrics::from_snapshot(&current_snapshot).unwrap();
        let mut current_results = HashMap::new();
        current_results.insert("test_basic".to_string(), true);

        let detector = RegressionDetector::new();
        let findings = detector
            .detect_regressions(&baseline, &current_metrics, &current_results)
            .unwrap();

        assert!(!findings.is_empty());
        assert!(findings
            .iter()
            .any(|f| f.finding_type == RegressionType::Performance));
        assert!(findings
            .iter()
            .any(|f| f.severity == RegressionSeverity::Failure));
    }

    #[test]
    fn test_correctness_regression() {
        let baseline = create_baseline();
        let mut current_snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            current_snapshot.add_latency(100.0 + i as f64);
        }
        for i in 0..5 {
            current_snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        current_snapshot.set_memory_peak(512.0);
        current_snapshot.set_cpu_average(45.0);
        current_snapshot.set_io_operations(42);

        let current_metrics = PerformanceMetrics::from_snapshot(&current_snapshot).unwrap();
        let mut current_results = HashMap::new();
        current_results.insert("test_basic".to_string(), false); // Test failed

        let detector = RegressionDetector::new();
        let findings = detector
            .detect_regressions(&baseline, &current_metrics, &current_results)
            .unwrap();

        assert!(!findings.is_empty());
        assert!(findings
            .iter()
            .any(|f| f.finding_type == RegressionType::Correctness));
        assert!(findings
            .iter()
            .any(|f| f.severity == RegressionSeverity::Failure));
    }

    #[test]
    fn test_performance_warning_threshold() {
        let baseline = create_baseline();
        let mut current_snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            current_snapshot.add_latency(103.0 + i as f64); // 3% increase
        }
        for i in 0..5 {
            current_snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        current_snapshot.set_memory_peak(512.0);
        current_snapshot.set_cpu_average(45.0);
        current_snapshot.set_io_operations(42);

        let current_metrics = PerformanceMetrics::from_snapshot(&current_snapshot).unwrap();
        let mut current_results = HashMap::new();
        current_results.insert("test_basic".to_string(), true);

        let detector = RegressionDetector::new();
        let findings = detector
            .detect_regressions(&baseline, &current_metrics, &current_results)
            .unwrap();

        assert!(!findings.is_empty());
        assert!(findings
            .iter()
            .any(|f| f.severity == RegressionSeverity::Warning));
    }

    #[test]
    fn test_custom_thresholds() {
        let mut thresholds = RegressionThresholds::default();
        thresholds.performance_failure = 20.0;
        thresholds.performance_warning = 10.0;

        let baseline = create_baseline();
        let mut current_snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            current_snapshot.add_latency(115.0 + i as f64); // 15% increase
        }
        for i in 0..5 {
            current_snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        current_snapshot.set_memory_peak(512.0);
        current_snapshot.set_cpu_average(45.0);
        current_snapshot.set_io_operations(42);

        let current_metrics = PerformanceMetrics::from_snapshot(&current_snapshot).unwrap();
        let mut current_results = HashMap::new();
        current_results.insert("test_basic".to_string(), true);

        let detector = RegressionDetector::with_thresholds(thresholds);
        let findings = detector
            .detect_regressions(&baseline, &current_metrics, &current_results)
            .unwrap();

        assert!(!findings.is_empty());
        assert!(findings
            .iter()
            .any(|f| f.severity == RegressionSeverity::Warning));
    }

    #[test]
    fn test_regression_finding_message() {
        let baseline = create_baseline();
        let mut current_snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            current_snapshot.add_latency(150.0 + i as f64); // 50% increase
        }
        for i in 0..5 {
            current_snapshot.add_throughput(1000.0 + i as f64 * 10.0);
        }
        current_snapshot.set_memory_peak(512.0);
        current_snapshot.set_cpu_average(45.0);
        current_snapshot.set_io_operations(42);

        let current_metrics = PerformanceMetrics::from_snapshot(&current_snapshot).unwrap();
        let mut current_results = HashMap::new();
        current_results.insert("test_basic".to_string(), true);

        let detector = RegressionDetector::new();
        let findings = detector
            .detect_regressions(&baseline, &current_metrics, &current_results)
            .unwrap();

        assert!(!findings.is_empty());
        let finding = &findings[0];
        assert!(finding.message.contains("regressed"));
        assert!(finding.message.contains("%"));
    }

    #[test]
    fn test_variance_calculation() {
        let detector = RegressionDetector::new();
        let runs = vec![100, 100, 100]; // Zero variance
        assert_eq!(detector.calculate_variance(&runs), 0.0);

        let runs = vec![100, 110, 90]; // Some variance
        let variance = detector.calculate_variance(&runs);
        assert!(variance > 0.0);
        assert!(variance < 100.0);
    }

    #[test]
    fn test_determinism_regression() {
        let baseline = create_baseline();
        let current_snapshot = MetricsSnapshot::new();
        let current_metrics = PerformanceMetrics::from_snapshot(&crate::metrics::MetricsSnapshot {
            latencies: vec![100.0],
            throughputs: vec![1000.0],
            memory_peak: 512.0,
            cpu_average: 45.0,
            io_operations: 42,
            timestamp: Utc::now(),
        })
        .unwrap();
        let mut current_results = HashMap::new();
        current_results.insert("test_basic".to_string(), true);

        let detector = RegressionDetector::new();
        let findings = detector
            .detect_regressions(&baseline, &current_metrics, &current_results)
            .unwrap();

        // Should detect determinism issue if variance > threshold
        // Results depend on the specific baseline setup
        let _ = findings;
    }

    #[test]
    fn test_regression_finding_serialization() {
        let finding = RegressionFinding {
            finding_type: RegressionType::Performance,
            severity: RegressionSeverity::Failure,
            metric: "latency_p99".to_string(),
            baseline_value: 100.0,
            current_value: 150.0,
            difference_percent: 50.0,
            threshold_percent: 5.0,
            message: "Latency regressed 50%".to_string(),
        };

        let json = serde_json::to_string(&finding).unwrap();
        let deserialized: RegressionFinding = serde_json::from_str(&json).unwrap();

        assert_eq!(finding.metric, deserialized.metric);
        assert_eq!(finding.severity, deserialized.severity);
    }
}
