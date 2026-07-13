//! Result collection and regression detection.

use crate::baseline::{Baseline, MetricEntry};
use crate::error::{OrchestratorError, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn, info};
use uuid::Uuid;

/// Test execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Unique result ID.
    pub id: Uuid,
    /// Job ID this result came from.
    pub job_id: Uuid,
    /// Worker that executed the test.
    pub worker_id: String,
    /// Test name.
    pub test_name: String,
    /// Execution status.
    pub status: ExecutionStatus,
    /// Metrics collected during execution.
    pub metrics: Vec<MetricEntry>,
    /// When execution started.
    pub started_at: DateTime<Utc>,
    /// When execution finished.
    pub finished_at: DateTime<Utc>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Failed,
    Timeout,
}

impl ExecutionStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionStatus::Success)
    }
}

/// Regression detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    /// Metric that regressed.
    pub metric: String,
    /// Baseline value.
    pub baseline_value: f64,
    /// Current value.
    pub current_value: f64,
    /// Percentage change.
    pub percent_change: f64,
    /// Severity (0.0 to 1.0).
    pub severity: f64,
}

impl RegressionReport {
    /// Determine if this is a severe regression (>10% degradation).
    pub fn is_severe(&self) -> bool {
        self.severity > 0.1
    }
}

/// Result collection and analysis.
pub struct ResultCollector {
    results: Arc<DashMap<Uuid, TestResult>>,
    regressions: Arc<DashMap<String, Vec<RegressionReport>>>,
}

impl ResultCollector {
    /// Create a new result collector.
    pub fn new() -> Self {
        Self {
            results: Arc::new(DashMap::new()),
            regressions: Arc::new(DashMap::new()),
        }
    }

    /// Record a test result.
    pub fn record_result(&self, result: TestResult) -> Result<()> {
        let id = result.id;
        self.results.insert(id, result);
        debug!("recorded result: {}", id);
        Ok(())
    }

    /// Get a result by ID.
    pub fn get_result(&self, id: Uuid) -> Result<TestResult> {
        self.results
            .get(&id)
            .map(|r| r.clone())
            .ok_or(OrchestratorError::ResultCollectionError(
                "result not found".to_string(),
            ))
    }

    /// Get all results for a job.
    pub fn get_job_results(&self, job_id: Uuid) -> Vec<TestResult> {
        self.results
            .iter()
            .filter(|r| r.value().job_id == job_id)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Compare test result against baseline.
    pub fn compare_against_baseline(&self, result: &TestResult, baseline: &Baseline) -> Result<ComparisonReport> {
        let mut regressions = Vec::new();

        for metric in &result.metrics {
            if let Some(baseline_metric) = baseline.get_metric(&metric.name) {
                let (percent_change, severity) =
                    Self::compute_regression(baseline_metric.value, metric.value, &metric.unit);

                if severity > 0.0 {
                    regressions.push(RegressionReport {
                        metric: metric.name.clone(),
                        baseline_value: baseline_metric.value,
                        current_value: metric.value,
                        percent_change,
                        severity,
                    });
                }
            }
        }

        let has_regression = !regressions.is_empty();
        if has_regression {
            let baseline_name = baseline.name.clone();
            self.regressions
                .insert(baseline_name, regressions.clone());
        }

        Ok(ComparisonReport {
            test_name: result.test_name.clone(),
            baseline_name: baseline.name.clone(),
            baseline_version: baseline.version,
            regressions,
            has_regression,
            timestamp: Utc::now(),
        })
    }

    /// Compute regression metric.
    /// Returns (percent_change, severity).
    /// For latency/time metrics, higher is worse.
    /// For throughput metrics, lower is worse.
    fn compute_regression(baseline: f64, current: f64, unit: &str) -> (f64, f64) {
        if baseline == 0.0 {
            return (0.0, 0.0);
        }

        let percent_change = ((current - baseline) / baseline) * 100.0;
        let is_time_metric = unit.contains("ms") || unit.contains("s") || unit.contains("us");

        let severity = if is_time_metric {
            // For time: higher is worse
            if current > baseline {
                (percent_change / 100.0).min(1.0).max(0.0)
            } else {
                0.0 // Improvement
            }
        } else {
            // For throughput/count: lower is worse
            if current < baseline {
                ((baseline - current) / baseline).min(1.0).max(0.0)
            } else {
                0.0 // Improvement
            }
        };

        (percent_change, severity)
    }

    /// Get all regressions for a baseline.
    pub fn get_regressions(&self, baseline_name: &str) -> Option<Vec<RegressionReport>> {
        self.regressions
            .get(baseline_name)
            .map(|r| r.clone())
    }

    /// Get all results.
    pub fn all_results(&self) -> Vec<TestResult> {
        self.results
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get results by status.
    pub fn results_by_status(&self, status: ExecutionStatus) -> Vec<TestResult> {
        self.results
            .iter()
            .filter(|r| r.value().status == status)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get results for a worker.
    pub fn worker_results(&self, worker_id: &str) -> Vec<TestResult> {
        self.results
            .iter()
            .filter(|r| r.value().worker_id == worker_id)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get collection statistics.
    pub fn statistics(&self) -> CollectionStatistics {
        let all_results = self.all_results();
        let success_count = self
            .results_by_status(ExecutionStatus::Success)
            .len();
        let failed_count = self
            .results_by_status(ExecutionStatus::Failed)
            .len();
        let timeout_count = self
            .results_by_status(ExecutionStatus::Timeout)
            .len();

        let total = all_results.len();
        let success_rate = if total > 0 {
            (success_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let avg_duration = if total > 0 {
            let total_ms: i64 = all_results
                .iter()
                .map(|r| (r.finished_at - r.started_at).num_milliseconds())
                .sum();
            (total_ms / total as i64) as u64
        } else {
            0
        };

        CollectionStatistics {
            total_results: total,
            success_count,
            failed_count,
            timeout_count,
            success_rate,
            avg_duration_ms: avg_duration,
            regression_count: self.regressions.len(),
        }
    }

    /// Clear all results.
    pub fn clear(&self) {
        self.results.clear();
        self.regressions.clear();
    }
}

impl Default for ResultCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison report between test result and baseline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub test_name: String,
    pub baseline_name: String,
    pub baseline_version: crate::baseline::BaselineVersion,
    pub regressions: Vec<RegressionReport>,
    pub has_regression: bool,
    pub timestamp: DateTime<Utc>,
}

/// Result collection statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionStatistics {
    pub total_results: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub timeout_count: usize,
    pub success_rate: f64,
    pub avg_duration_ms: u64,
    pub regression_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_creation() {
        let result = TestResult {
            id: Uuid::new_v4(),
            job_id: Uuid::new_v4(),
            worker_id: "w1".to_string(),
            test_name: "test".to_string(),
            status: ExecutionStatus::Success,
            metrics: vec![],
            started_at: Utc::now(),
            finished_at: Utc::now(),
            error: None,
            metadata: HashMap::new(),
        };

        assert_eq!(result.status, ExecutionStatus::Success);
        assert!(result.status.is_success());
    }

    #[test]
    fn test_regression_detection() {
        // Time metric: current > baseline = regression
        let (pct, sev) = ResultCollector::compute_regression(100.0, 150.0, "ms");
        assert!(pct > 0.0);
        assert!(sev > 0.0);

        // Throughput: current < baseline = regression
        let (pct, sev) = ResultCollector::compute_regression(1000.0, 800.0, "ops/sec");
        assert!(pct < 0.0);
        assert!(sev > 0.0);
    }

    #[test]
    fn test_result_collector() {
        let collector = ResultCollector::new();
        let result = TestResult {
            id: Uuid::new_v4(),
            job_id: Uuid::new_v4(),
            worker_id: "w1".to_string(),
            test_name: "test".to_string(),
            status: ExecutionStatus::Success,
            metrics: vec![],
            started_at: Utc::now(),
            finished_at: Utc::now(),
            error: None,
            metadata: HashMap::new(),
        };

        let id = result.id;
        collector.record_result(result).unwrap();
        let retrieved = collector.get_result(id).unwrap();
        assert_eq!(retrieved.id, id);
    }
}
