//! Result capture for test execution

use crate::metrics::PerformanceMetrics;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

/// Test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultStatus {
    /// Test passed
    Passed,
    /// Test failed
    Failed,
    /// Test timed out
    Timeout,
    /// Test was skipped
    Skipped,
    /// Test crashed
    Crashed,
}

impl std::fmt::Display for ResultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passed => write!(f, "Passed"),
            Self::Failed => write!(f, "Failed"),
            Self::Timeout => write!(f, "Timeout"),
            Self::Skipped => write!(f, "Skipped"),
            Self::Crashed => write!(f, "Crashed"),
        }
    }
}

/// Test result output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Return code
    pub return_code: i32,
}

/// Test result capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultCapture {
    /// Vault ID where test executed
    pub vault_id: Uuid,
    /// Test binary path
    pub binary_path: PathBuf,
    /// Execution status
    pub status: ResultStatus,
    /// Test output
    pub output: Option<ResultOutput>,
    /// Performance metrics
    pub metrics: Option<PerformanceMetrics>,
    /// Start time
    pub start_time: Option<DateTime<Utc>>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Execution timeout
    pub timeout: Duration,
    /// Test logs
    pub logs: Vec<String>,
    /// Trace data (for replay)
    pub trace_data: Option<Vec<u8>>,
}

impl ResultCapture {
    /// Create a new result capture
    pub fn new(vault_id: Uuid, binary_path: PathBuf, timeout: Duration) -> Self {
        Self {
            vault_id,
            binary_path,
            status: ResultStatus::Passed,
            output: None,
            metrics: None,
            start_time: Some(Utc::now()),
            end_time: None,
            timeout,
            logs: Vec::new(),
            trace_data: None,
        }
    }

    /// Set the result status
    pub fn set_status(&mut self, status: ResultStatus) {
        self.status = status;
    }

    /// Set output
    pub fn set_output(&mut self, output: ResultOutput) {
        self.output = Some(output);
    }

    /// Set metrics
    pub fn set_metrics(&mut self, metrics: PerformanceMetrics) {
        self.metrics = Some(metrics);
    }

    /// Add a log line
    pub fn add_log(&mut self, log: String) {
        self.logs.push(log);
    }

    /// Add multiple log lines
    pub fn add_logs(&mut self, logs: Vec<String>) {
        self.logs.extend(logs);
    }

    /// Mark as completed
    pub fn mark_completed(&mut self) {
        self.end_time = Some(Utc::now());
    }

    /// Get execution duration
    pub fn duration(&self) -> Option<Duration> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => {
                let duration = end.signed_duration_since(start);
                Some(Duration::from_millis(
                    duration.num_milliseconds() as u64
                ))
            }
            _ => None,
        }
    }

    /// Check if test passed
    pub fn is_passed(&self) -> bool {
        self.status == ResultStatus::Passed
    }

    /// Check if test failed
    pub fn is_failed(&self) -> bool {
        matches!(
            self.status,
            ResultStatus::Failed | ResultStatus::Timeout | ResultStatus::Crashed
        )
    }

    /// Check if result timed out
    pub fn is_timeout(&self) -> bool {
        self.status == ResultStatus::Timeout
    }

    /// Get logs as a single string
    pub fn logs_as_string(&self) -> String {
        self.logs.join("\n")
    }
}

/// Aggregated test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResults {
    /// Total tests
    pub total: usize,
    /// Passed tests
    pub passed: usize,
    /// Failed tests
    pub failed: usize,
    /// Timed out tests
    pub timeout: usize,
    /// Skipped tests
    pub skipped: usize,
    /// Crashed tests
    pub crashed: usize,
}

impl AggregatedResults {
    /// Create from result captures
    pub fn from_captures(results: &[ResultCapture]) -> Self {
        let mut agg = Self {
            total: results.len(),
            passed: 0,
            failed: 0,
            timeout: 0,
            skipped: 0,
            crashed: 0,
        };

        for result in results {
            match result.status {
                ResultStatus::Passed => agg.passed += 1,
                ResultStatus::Failed => agg.failed += 1,
                ResultStatus::Timeout => agg.timeout += 1,
                ResultStatus::Skipped => agg.skipped += 1,
                ResultStatus::Crashed => agg.crashed += 1,
            }
        }

        agg
    }

    /// Calculate pass rate
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.passed as f64 / self.total as f64) * 100.0
    }

    /// Calculate failure rate
    pub fn failure_rate(&self) -> f64 {
        100.0 - self.pass_rate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_status_display() {
        assert_eq!(ResultStatus::Passed.to_string(), "Passed");
        assert_eq!(ResultStatus::Failed.to_string(), "Failed");
        assert_eq!(ResultStatus::Timeout.to_string(), "Timeout");
    }

    #[test]
    fn test_result_capture_creation() {
        let vault_id = Uuid::new_v4();
        let binary_path = PathBuf::from("/path/to/test");
        let timeout = Duration::from_secs(60);

        let result = ResultCapture::new(vault_id, binary_path.clone(), timeout);
        assert_eq!(result.vault_id, vault_id);
        assert_eq!(result.binary_path, binary_path);
        assert_eq!(result.status, ResultStatus::Passed);
    }

    #[test]
    fn test_result_capture_logs() {
        let vault_id = Uuid::new_v4();
        let binary_path = PathBuf::from("/path/to/test");
        let timeout = Duration::from_secs(60);

        let mut result = ResultCapture::new(vault_id, binary_path, timeout);
        result.add_log("Test log 1".to_string());
        result.add_log("Test log 2".to_string());

        assert_eq!(result.logs.len(), 2);
        let logs_str = result.logs_as_string();
        assert!(logs_str.contains("Test log 1"));
    }

    #[test]
    fn test_result_capture_is_passed() {
        let vault_id = Uuid::new_v4();
        let binary_path = PathBuf::from("/path/to/test");
        let timeout = Duration::from_secs(60);

        let mut result = ResultCapture::new(vault_id, binary_path, timeout);
        assert!(result.is_passed());

        result.set_status(ResultStatus::Failed);
        assert!(!result.is_passed());
        assert!(result.is_failed());
    }

    #[test]
    fn test_aggregated_results() {
        let mut results = Vec::new();
        for _ in 0..8 {
            let result = ResultCapture::new(Uuid::new_v4(), PathBuf::from("/test"), Duration::from_secs(60));
            results.push(result);
        }

        // Modify some to failed
        for i in 0..2 {
            results[i].set_status(ResultStatus::Failed);
        }

        let agg = AggregatedResults::from_captures(&results);
        assert_eq!(agg.total, 8);
        assert_eq!(agg.passed, 6);
        assert_eq!(agg.failed, 2);
        assert_eq!(agg.pass_rate(), 75.0);
    }
}
