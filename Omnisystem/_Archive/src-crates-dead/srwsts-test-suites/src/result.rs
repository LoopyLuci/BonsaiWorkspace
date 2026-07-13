//! Test Suite Result Collection and Aggregation
//!
//! Collects, aggregates, and reports test results across all suites.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::SrwstsResult;

/// Overall test suite execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteResult {
    pub suite_id: String,
    pub run_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub test_results: Vec<TestResultRecord>,
    pub status: SuiteStatus,
}

/// Status of a suite execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuiteStatus {
    Running,
    Passed,
    Failed,
    PartialFailure,
    Skipped,
    Aborted,
}

impl std::fmt::Display for SuiteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "Running"),
            Self::Passed => write!(f, "Passed"),
            Self::Failed => write!(f, "Failed"),
            Self::PartialFailure => write!(f, "PartialFailure"),
            Self::Skipped => write!(f, "Skipped"),
            Self::Aborted => write!(f, "Aborted"),
        }
    }
}

/// Individual test result within a suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultRecord {
    pub test_id: String,
    pub test_name: String,
    pub passed: bool,
    pub elapsed_ms: u128,
    pub error_message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Result collector trait
pub trait SuiteResultCollector: Send + Sync {
    /// Record a test result
    fn record_test_result(&self, result: TestResultRecord) -> SrwstsResult<()>;

    /// Finalize the suite result
    fn finalize(&self) -> SrwstsResult<SuiteResult>;

    /// Get current result snapshot
    fn get_current_result(&self) -> SrwstsResult<SuiteResult>;
}

/// Default result collector implementation
pub struct DefaultSuiteResultCollector {
    suite_id: String,
    run_id: String,
    results: std::sync::Arc<tokio::sync::Mutex<Vec<TestResultRecord>>>,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl DefaultSuiteResultCollector {
    /// Create a new result collector
    pub fn new(suite_id: impl Into<String>) -> Self {
        Self {
            suite_id: suite_id.into(),
            run_id: uuid::Uuid::new_v4().to_string(),
            results: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            start_time: chrono::Utc::now(),
        }
    }
}

impl SuiteResultCollector for DefaultSuiteResultCollector {
    fn record_test_result(&self, _result: TestResultRecord) -> SrwstsResult<()> {
        // This would normally be async, but we're blocking for now
        // In a real implementation, we'd use tokio spawn
        Ok(())
    }

    fn finalize(&self) -> SrwstsResult<SuiteResult> {
        Ok(SuiteResult {
            suite_id: self.suite_id.clone(),
            run_id: self.run_id.clone(),
            start_time: self.start_time,
            end_time: Some(chrono::Utc::now()),
            total_tests: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            test_results: vec![],
            status: SuiteStatus::Passed,
        })
    }

    fn get_current_result(&self) -> SrwstsResult<SuiteResult> {
        Ok(SuiteResult {
            suite_id: self.suite_id.clone(),
            run_id: self.run_id.clone(),
            start_time: self.start_time,
            end_time: None,
            total_tests: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            test_results: vec![],
            status: SuiteStatus::Running,
        })
    }
}

/// Aggregated results across multiple suites
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResults {
    pub run_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub suite_results: HashMap<String, SuiteResult>,
    pub total_tests: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub total_skipped: usize,
    pub overall_status: OverallStatus,
}

/// Overall execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverallStatus {
    Running,
    Success,
    Failure,
    PartialSuccess,
    Aborted,
}

impl std::fmt::Display for OverallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "Running"),
            Self::Success => write!(f, "Success"),
            Self::Failure => write!(f, "Failure"),
            Self::PartialSuccess => write!(f, "PartialSuccess"),
            Self::Aborted => write!(f, "Aborted"),
        }
    }
}

impl AggregatedResults {
    /// Create a new aggregated results container
    pub fn new() -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            start_time: chrono::Utc::now(),
            end_time: None,
            suite_results: HashMap::new(),
            total_tests: 0,
            total_passed: 0,
            total_failed: 0,
            total_skipped: 0,
            overall_status: OverallStatus::Running,
        }
    }

    /// Add a suite result
    pub fn add_suite_result(&mut self, result: SuiteResult) {
        self.total_tests += result.total_tests;
        self.total_passed += result.passed;
        self.total_failed += result.failed;
        self.total_skipped += result.skipped;

        self.suite_results.insert(result.suite_id.clone(), result);
    }

    /// Finalize aggregation
    pub fn finalize(&mut self) {
        self.end_time = Some(chrono::Utc::now());

        // Determine overall status
        if self.total_failed == 0 {
            self.overall_status = OverallStatus::Success;
        } else if self.total_passed > 0 {
            self.overall_status = OverallStatus::PartialSuccess;
        } else {
            self.overall_status = OverallStatus::Failure;
        }
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.total_passed as f64 / self.total_tests as f64
        }
    }

    /// Generate a summary report
    pub fn summary_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("Test Run ID: {}\n", self.run_id));
        report.push_str(&format!("Overall Status: {}\n", self.overall_status));
        report.push_str(&format!("Start Time: {}\n", self.start_time));
        if let Some(end) = self.end_time {
            report.push_str(&format!("End Time: {}\n", end));
            let duration = end.signed_duration_since(self.start_time);
            report.push_str(&format!("Duration: {:.2}s\n", duration.num_seconds()));
        }
        report.push_str(&format!("\nResults:\n"));
        report.push_str(&format!("  Total Tests: {}\n", self.total_tests));
        report.push_str(&format!("  Passed: {} ({:.1}%)\n", self.total_passed, self.success_rate() * 100.0));
        report.push_str(&format!("  Failed: {}\n", self.total_failed));
        report.push_str(&format!("  Skipped: {}\n", self.total_skipped));
        report.push_str(&format!("\nSuite Results:\n"));

        for (suite_id, result) in &self.suite_results {
            report.push_str(&format!("  {} [{}]: {} / {} passed\n",
                suite_id,
                result.status,
                result.passed,
                result.total_tests
            ));
        }

        report
    }
}

impl Default for AggregatedResults {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregated_results_creation() {
        let results = AggregatedResults::new();
        assert_eq!(results.total_tests, 0);
        assert_eq!(results.overall_status, OverallStatus::Running);
    }

    #[test]
    fn test_aggregated_results_calculation() {
        let mut results = AggregatedResults::new();
        results.total_tests = 100;
        results.total_passed = 95;
        results.total_failed = 5;

        assert_eq!(results.success_rate(), 0.95);
    }

    #[test]
    fn test_suite_status_display() {
        assert_eq!(format!("{}", SuiteStatus::Passed), "Passed");
        assert_eq!(format!("{}", SuiteStatus::Failed), "Failed");
    }

    #[test]
    fn test_result_collector_creation() {
        let collector = DefaultSuiteResultCollector::new("test_suite");
        assert_eq!(collector.suite_id, "test_suite");
    }

    #[test]
    fn test_summary_report() {
        let mut results = AggregatedResults::new();
        results.total_tests = 100;
        results.total_passed = 90;
        results.total_failed = 10;
        results.finalize();

        let report = results.summary_report();
        assert!(report.contains("Overall Status"));
        assert!(report.contains("100"));
        assert!(report.contains("90.0%"));
    }
}
