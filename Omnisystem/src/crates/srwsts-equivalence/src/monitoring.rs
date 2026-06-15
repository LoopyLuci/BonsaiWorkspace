//! Continuous equivalence monitoring system
//!
//! Tracks architecture divergence over time and prevents regression.

use crate::EquivalenceReport;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Test result history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultHistory {
    /// Test name
    pub test_name: String,
    /// Test results over time
    pub results: Vec<TestResult>,
    /// Current status
    pub current_status: TestStatus,
    /// Last divergence date (if any)
    pub last_divergence: Option<DateTime<Utc>>,
}

impl TestResultHistory {
    /// Create a new history
    pub fn new(test_name: String) -> Self {
        Self {
            test_name,
            results: Vec::new(),
            current_status: TestStatus::Unknown,
            last_divergence: None,
        }
    }

    /// Add a test result
    pub fn add_result(&mut self, result: TestResult) {
        if result.status == EquivalenceStatus::Red {
            self.last_divergence = Some(Utc::now());
        }
        self.current_status = match result.status {
            EquivalenceStatus::Green => TestStatus::Passing,
            EquivalenceStatus::Yellow => TestStatus::Warning,
            EquivalenceStatus::Red => TestStatus::Failing,
            EquivalenceStatus::Unknown => TestStatus::Unknown,
        };
        self.results.push(result);
    }

    /// Check for regression (previous pass, now failing)
    pub fn detect_regression(&self) -> bool {
        if self.results.len() < 2 {
            return false;
        }

        let previous = &self.results[self.results.len() - 2];
        let current = &self.results[self.results.len() - 1];

        previous.status == EquivalenceStatus::Green && current.status != EquivalenceStatus::Green
    }

    /// Get trend over last N results
    pub fn get_trend(&self, window_size: usize) -> Vec<EquivalenceStatus> {
        let start = if self.results.len() > window_size {
            self.results.len() - window_size
        } else {
            0
        };

        self.results[start..]
            .iter()
            .map(|r| r.status)
            .collect()
    }
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Status
    pub status: EquivalenceStatus,
    /// Architectures tested
    pub architectures: Vec<String>,
    /// Performance delta (percent difference from baseline)
    pub performance_delta_percent: f64,
    /// Notes
    pub notes: String,
}

// Re-export EquivalenceStatus from equivalence module
use crate::EquivalenceStatus;

/// Test status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    /// Test is passing
    Passing,
    /// Test has warnings
    Warning,
    /// Test is failing
    Failing,
    /// Unknown status
    Unknown,
}

impl From<crate::EquivalenceStatus> for TestStatus {
    fn from(status: crate::EquivalenceStatus) -> Self {
        match status {
            crate::EquivalenceStatus::Green => TestStatus::Passing,
            crate::EquivalenceStatus::Yellow => TestStatus::Warning,
            crate::EquivalenceStatus::Red => TestStatus::Failing,
            crate::EquivalenceStatus::Unknown => TestStatus::Unknown,
        }
    }
}

/// Equivalence monitoring system
pub struct EquivalenceMonitor {
    /// Test histories
    histories: Arc<DashMap<String, TestResultHistory>>,
    /// Architecture baselines
    baselines: Arc<DashMap<String, ArchitectureBaseline>>,
}

impl EquivalenceMonitor {
    /// Create a new monitor
    pub fn new() -> Self {
        Self {
            histories: Arc::new(DashMap::new()),
            baselines: Arc::new(DashMap::new()),
        }
    }

    /// Record a test result
    pub async fn record_test(&self, report: &EquivalenceReport) {
        let status = match report.status {
            crate::EquivalenceStatus::Green => EquivalenceStatus::Green,
            crate::EquivalenceStatus::Yellow => EquivalenceStatus::Yellow,
            crate::EquivalenceStatus::Red => EquivalenceStatus::Red,
            crate::EquivalenceStatus::Unknown => EquivalenceStatus::Unknown,
        };

        let result = TestResult {
            timestamp: Utc::now(),
            status,
            architectures: vec![],
            performance_delta_percent: 0.0,
            notes: report.summary.clone(),
        };

        let mut history = self
            .histories
            .entry(report.test_name.clone())
            .or_insert_with(|| TestResultHistory::new(report.test_name.clone()));

        history.add_result(result);
    }

    /// Get test history
    pub async fn get_history(&self, test_name: &str) -> Option<TestResultHistory> {
        self.histories.get(test_name).map(|h| h.clone())
    }

    /// Check for regressions
    pub async fn detect_regressions(&self) -> Vec<RegressionAlert> {
        let mut alerts = Vec::new();

        for entry in self.histories.iter() {
            if entry.detect_regression() {
                alerts.push(RegressionAlert {
                    test_name: entry.test_name.clone(),
                    detected_at: Utc::now(),
                    previous_status: "PASS".to_string(),
                    current_status: "FAIL".to_string(),
                });
            }
        }

        alerts
    }

    /// Set baseline for an architecture
    pub async fn set_baseline(&self, arch: String, baseline: ArchitectureBaseline) {
        self.baselines.insert(arch, baseline);
    }

    /// Get baseline for architecture
    pub async fn get_baseline(&self, arch: &str) -> Option<ArchitectureBaseline> {
        self.baselines.get(arch).map(|b| b.clone())
    }

    /// Get overall health status
    pub async fn get_health_status(&self) -> HealthStatus {
        let total_tests = self.histories.len();
        let mut passing = 0;
        let mut warning = 0;
        let mut failing = 0;

        for entry in self.histories.iter() {
            match entry.current_status {
                TestStatus::Passing => passing += 1,
                TestStatus::Warning => warning += 1,
                TestStatus::Failing => failing += 1,
                TestStatus::Unknown => {}
            }
        }

        let health_score = if total_tests == 0 {
            100.0
        } else {
            (passing as f64 / total_tests as f64) * 100.0
        };

        HealthStatus {
            total_tests,
            passing,
            warning,
            failing,
            health_score,
            status: if failing > 0 {
                "CRITICAL".to_string()
            } else if warning > 0 {
                "DEGRADED".to_string()
            } else {
                "HEALTHY".to_string()
            },
        }
    }

    /// Get trend for a test
    pub async fn get_trend(&self, test_name: &str, window_size: usize) -> Option<Vec<EquivalenceStatus>> {
        self.histories
            .get(test_name)
            .map(|h| h.get_trend(window_size))
    }
}

impl Default for EquivalenceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Architecture baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureBaseline {
    /// Architecture name
    pub architecture: String,
    /// Baseline latency in nanoseconds
    pub baseline_latency_ns: u64,
    /// Baseline L1 hit ratio
    pub baseline_l1_hit_ratio: f64,
    /// Baseline L2 hit ratio
    pub baseline_l2_hit_ratio: f64,
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

impl ArchitectureBaseline {
    /// Create a new baseline
    pub fn new(
        architecture: String,
        baseline_latency_ns: u64,
        baseline_l1_hit_ratio: f64,
        baseline_l2_hit_ratio: f64,
    ) -> Self {
        Self {
            architecture,
            baseline_latency_ns,
            baseline_l1_hit_ratio,
            baseline_l2_hit_ratio,
            last_updated: Utc::now(),
        }
    }
}

/// Regression alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAlert {
    /// Test name
    pub test_name: String,
    /// When detected
    pub detected_at: DateTime<Utc>,
    /// Previous status
    pub previous_status: String,
    /// Current status
    pub current_status: String,
}

/// Overall health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Total tests
    pub total_tests: usize,
    /// Passing tests
    pub passing: usize,
    /// Tests with warnings
    pub warning: usize,
    /// Failing tests
    pub failing: usize,
    /// Health score (0-100)
    pub health_score: f64,
    /// Overall status
    pub status: String,
}

impl HealthStatus {
    /// Check if system is healthy
    pub fn is_healthy(&self) -> bool {
        self.failing == 0 && self.health_score >= 95.0
    }

    /// Check if system is degraded
    pub fn is_degraded(&self) -> bool {
        self.warning > 0 || (self.health_score < 95.0 && self.health_score >= 80.0)
    }

    /// Check if system is critical
    pub fn is_critical(&self) -> bool {
        self.failing > 0 || self.health_score < 80.0
    }
}

/// Monitoring report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringReport {
    /// Generated at
    pub generated_at: DateTime<Utc>,
    /// Health status
    pub health: HealthStatus,
    /// Recent regressions
    pub regressions: Vec<RegressionAlert>,
    /// Test histories (summary)
    pub test_summaries: Vec<TestSummary>,
}

/// Summary of a test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    /// Test name
    pub test_name: String,
    /// Current status
    pub status: String,
    /// Result count
    pub result_count: usize,
    /// Last run timestamp
    pub last_run: Option<DateTime<Utc>>,
    /// Success rate (percent)
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_history_creation() {
        let history = TestResultHistory::new("test".to_string());
        assert_eq!(history.results.len(), 0);
        assert_eq!(history.current_status, TestStatus::Unknown);
    }

    #[test]
    fn test_regression_detection() {
        let mut history = TestResultHistory::new("test".to_string());

        history.add_result(TestResult {
            timestamp: Utc::now(),
            status: EquivalenceStatus::Green,
            architectures: vec![],
            performance_delta_percent: 0.0,
            notes: "".to_string(),
        });

        history.add_result(TestResult {
            timestamp: Utc::now(),
            status: EquivalenceStatus::Red,
            architectures: vec![],
            performance_delta_percent: 0.0,
            notes: "".to_string(),
        });

        assert!(history.detect_regression());
    }

    #[test]
    fn test_trend() {
        let mut history = TestResultHistory::new("test".to_string());

        for status in vec![
            EquivalenceStatus::Green,
            EquivalenceStatus::Green,
            EquivalenceStatus::Yellow,
        ] {
            history.add_result(TestResult {
                timestamp: Utc::now(),
                status,
                architectures: vec![],
                performance_delta_percent: 0.0,
                notes: "".to_string(),
            });
        }

        let trend = history.get_trend(3);
        assert_eq!(trend.len(), 3);
    }

    #[test]
    fn test_baseline_creation() {
        let baseline = ArchitectureBaseline::new(
            "x86_64".to_string(),
            1000,
            0.9,
            0.8,
        );

        assert_eq!(baseline.architecture, "x86_64");
        assert_eq!(baseline.baseline_latency_ns, 1000);
    }

    #[test]
    fn test_health_status_healthy() {
        let health = HealthStatus {
            total_tests: 10,
            passing: 10,
            warning: 0,
            failing: 0,
            health_score: 100.0,
            status: "HEALTHY".to_string(),
        };

        assert!(health.is_healthy());
    }

    #[test]
    fn test_health_status_degraded() {
        let health = HealthStatus {
            total_tests: 10,
            passing: 8,
            warning: 2,
            failing: 0,
            health_score: 85.0,
            status: "DEGRADED".to_string(),
        };

        assert!(health.is_degraded());
    }

    #[test]
    fn test_health_status_critical() {
        let health = HealthStatus {
            total_tests: 10,
            passing: 5,
            warning: 0,
            failing: 5,
            health_score: 50.0,
            status: "CRITICAL".to_string(),
        };

        assert!(health.is_critical());
    }
}
