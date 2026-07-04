//! Result Reporting
//!
//! Generates comprehensive test reports with pass/fail status, metrics, detailed logs,
//! and analysis. Outputs JSON for machine parsing and human-readable summaries.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use anyhow::Result;
use tracing::info;

/// Test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Pass,
    Fail,
    Skip,
    Timeout,
    Error,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Fail => write!(f, "FAIL"),
            Self::Skip => write!(f, "SKIP"),
            Self::Timeout => write!(f, "TIMEOUT"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseResult {
    pub name: String,
    pub status: String,
    pub duration_ms: f64,
    pub message: Option<String>,
    pub metrics: serde_json::Value,
}

impl TestCaseResult {
    /// Create a new test case result
    pub fn new(name: impl Into<String>, status: TestStatus) -> Self {
        Self {
            name: name.into(),
            status: status.to_string(),
            duration_ms: 0.0,
            message: None,
            metrics: json!({}),
        }
    }

    /// Set duration
    pub fn with_duration(mut self, ms: f64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Set message
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    /// Set metrics
    pub fn with_metrics(mut self, metrics: serde_json::Value) -> Self {
        self.metrics = metrics;
        self
    }
}

/// Test suite result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteResult {
    pub name: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub duration_ms: f64,
    pub test_cases: Vec<TestCaseResult>,
}

impl TestSuiteResult {
    /// Create a new test suite result
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            skipped_tests: 0,
            duration_ms: 0.0,
            test_cases: Vec::new(),
        }
    }

    /// Add a test case
    pub fn add_test(&mut self, test: TestCaseResult) {
        match test.status.as_str() {
            "PASS" => self.passed_tests += 1,
            "FAIL" => self.failed_tests += 1,
            "SKIP" => self.skipped_tests += 1,
            _ => {}
        }
        self.total_tests += 1;
        self.test_cases.push(test);
    }

    /// Get pass rate as percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }

    /// Check if suite passed
    pub fn is_passed(&self) -> bool {
        self.failed_tests == 0
    }
}

/// Complete test report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultReport {
    pub test_run_id: String,
    pub timestamp: String,
    pub duration_ms: f64,
    pub test_suites: Vec<TestSuiteResult>,
    pub total_tests: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub overall_pass_rate: f64,
    pub kernel_version: String,
    pub system_info: serde_json::Value,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ResultReport {
    /// Create a new result report
    pub fn new(test_run_id: impl Into<String>, kernel_version: impl Into<String>) -> Self {
        let now = chrono::Local::now();

        Self {
            test_run_id: test_run_id.into(),
            timestamp: now.to_rfc3339(),
            duration_ms: 0.0,
            test_suites: Vec::new(),
            total_tests: 0,
            total_passed: 0,
            total_failed: 0,
            overall_pass_rate: 0.0,
            kernel_version: kernel_version.into(),
            system_info: json!({}),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a test suite result
    pub fn add_suite(&mut self, suite: TestSuiteResult) {
        self.total_tests += suite.total_tests;
        self.total_passed += suite.passed_tests;
        self.total_failed += suite.failed_tests;
        self.test_suites.push(suite);
    }

    /// Add an error
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Set system info
    pub fn set_system_info(&mut self, info: serde_json::Value) {
        self.system_info = info;
    }

    /// Calculate overall pass rate
    pub fn calculate_pass_rate(&mut self) {
        if self.total_tests == 0 {
            self.overall_pass_rate = 0.0;
        } else {
            self.overall_pass_rate = (self.total_passed as f64 / self.total_tests as f64) * 100.0;
        }
    }

    /// Check if all tests passed
    pub fn is_passed(&self) -> bool {
        self.total_failed == 0 && self.total_tests > 0
    }

    /// Get HTML summary
    pub fn to_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str("<title>Kernel Stress Test Report</title>\n");
        html.push_str("<style>");
        html.push_str("body { font-family: Arial, sans-serif; margin: 20px; }");
        html.push_str(".pass { color: green; font-weight: bold; }");
        html.push_str(".fail { color: red; font-weight: bold; }");
        html.push_str("table { border-collapse: collapse; width: 100%; margin: 20px 0; }");
        html.push_str("th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }");
        html.push_str("th { background-color: #f2f2f2; }");
        html.push_str("</style>\n");
        html.push_str("</head>\n<body>\n");

        html.push_str(&format!("<h1>UOSC Kernel Stress Test Report</h1>\n"));
        html.push_str(&format!("<p><strong>Test Run ID:</strong> {}</p>\n", self.test_run_id));
        html.push_str(&format!("<p><strong>Timestamp:</strong> {}</p>\n", self.timestamp));
        html.push_str(&format!("<p><strong>Duration:</strong> {:.2}ms</p>\n", self.duration_ms));
        html.push_str(&format!("<p><strong>Kernel Version:</strong> {}</p>\n", self.kernel_version));

        html.push_str("<h2>Summary</h2>\n");
        html.push_str("<table>\n");
        html.push_str("<tr><th>Total Tests</th><th>Passed</th><th>Failed</th><th>Pass Rate</th></tr>\n");
        html.push_str(&format!(
            "<tr><td>{}</td><td class='pass'>{}</td><td class='fail'>{}</td><td>{:.1}%</td></tr>\n",
            self.total_tests, self.total_passed, self.total_failed, self.overall_pass_rate
        ));
        html.push_str("</table>\n");

        html.push_str("<h2>Test Suites</h2>\n");
        for suite in &self.test_suites {
            html.push_str(&format!("<h3>{}</h3>\n", suite.name));
            html.push_str("<table>\n");
            html.push_str("<tr><th>Test</th><th>Status</th><th>Duration</th></tr>\n");

            for test in &suite.test_cases {
                let status_class = if test.status == "PASS" { "pass" } else { "fail" };
                html.push_str(&format!(
                    "<tr><td>{}</td><td class='{}'>{}</td><td>{:.2}ms</td></tr>\n",
                    test.name, status_class, test.status, test.duration_ms
                ));
            }

            html.push_str("</table>\n");
        }

        if !self.errors.is_empty() {
            html.push_str("<h2>Errors</h2>\n<ul>\n");
            for error in &self.errors {
                html.push_str(&format!("<li>{}</li>\n", error));
            }
            html.push_str("</ul>\n");
        }

        html.push_str("</body>\n</html>\n");
        html
    }

    /// Get text summary
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        text.push_str("=== UOSC Kernel Stress Test Report ===\n\n");
        text.push_str(&format!("Test Run ID: {}\n", self.test_run_id));
        text.push_str(&format!("Timestamp: {}\n", self.timestamp));
        text.push_str(&format!("Duration: {:.2}ms\n", self.duration_ms));
        text.push_str(&format!("Kernel Version: {}\n\n", self.kernel_version));

        text.push_str("Summary:\n");
        text.push_str(&format!("  Total Tests: {}\n", self.total_tests));
        text.push_str(&format!("  Passed: {}\n", self.total_passed));
        text.push_str(&format!("  Failed: {}\n", self.total_failed));
        text.push_str(&format!("  Pass Rate: {:.1}%\n\n", self.overall_pass_rate));

        text.push_str("Test Suites:\n");
        for suite in &self.test_suites {
            text.push_str(&format!(
                "  {}: {}/{} passed ({:.1}%)\n",
                suite.name, suite.passed_tests, suite.total_tests, suite.pass_rate()
            ));
        }

        if !self.errors.is_empty() {
            text.push_str("\nErrors:\n");
            for error in &self.errors {
                text.push_str(&format!("  - {}\n", error));
            }
        }

        if !self.warnings.is_empty() {
            text.push_str("\nWarnings:\n");
            for warning in &self.warnings {
                text.push_str(&format!("  - {}\n", warning));
            }
        }

        text
    }
}

/// Report generator
pub struct ReportGenerator;

impl ReportGenerator {
    /// Generate JSON report
    pub fn to_json(report: &ResultReport) -> Result<String> {
        Ok(serde_json::to_string_pretty(report)?)
    }

    /// Write report to file
    pub fn write_json_file(report: &ResultReport, path: &str) -> Result<()> {
        let json = Self::to_json(report)?;
        fs::write(path, json)?;
        info!("Report written to {}", path);
        Ok(())
    }

    /// Write HTML report to file
    pub fn write_html_file(report: &ResultReport, path: &str) -> Result<()> {
        let html = report.to_html();
        fs::write(path, html)?;
        info!("HTML report written to {}", path);
        Ok(())
    }

    /// Write text report to file
    pub fn write_text_file(report: &ResultReport, path: &str) -> Result<()> {
        let text = report.to_text();
        fs::write(path, text)?;
        info!("Text report written to {}", path);
        Ok(())
    }

    /// Write all report formats
    pub fn write_all_formats(report: &ResultReport, prefix: &str) -> Result<()> {
        Self::write_json_file(report, &format!("{}.json", prefix))?;
        Self::write_html_file(report, &format!("{}.html", prefix))?;
        Self::write_text_file(report, &format!("{}.txt", prefix))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_case_result_creation() {
        let result = TestCaseResult::new("test1", TestStatus::Pass);
        assert_eq!(result.name, "test1");
        assert_eq!(result.status, "PASS");
    }

    #[test]
    fn test_test_suite_result_creation() {
        let mut suite = TestSuiteResult::new("suite1");
        suite.add_test(TestCaseResult::new("test1", TestStatus::Pass));
        suite.add_test(TestCaseResult::new("test2", TestStatus::Fail));

        assert_eq!(suite.total_tests, 2);
        assert_eq!(suite.passed_tests, 1);
        assert_eq!(suite.failed_tests, 1);
        assert_eq!(suite.pass_rate(), 50.0);
    }

    #[test]
    fn test_result_report_creation() {
        let report = ResultReport::new("run-1", "UOSC-0.1.0");
        assert_eq!(report.test_run_id, "run-1");
        assert_eq!(report.kernel_version, "UOSC-0.1.0");
        assert_eq!(report.total_tests, 0);
    }

    #[test]
    fn test_result_report_add_suite() {
        let mut report = ResultReport::new("run-1", "UOSC-0.1.0");
        let mut suite = TestSuiteResult::new("suite1");
        suite.add_test(TestCaseResult::new("test1", TestStatus::Pass));

        report.add_suite(suite);

        assert_eq!(report.total_tests, 1);
        assert_eq!(report.total_passed, 1);
    }

    #[test]
    fn test_result_report_pass_rate() {
        let mut report = ResultReport::new("run-1", "UOSC-0.1.0");
        let mut suite = TestSuiteResult::new("suite1");
        suite.add_test(TestCaseResult::new("test1", TestStatus::Pass));
        suite.add_test(TestCaseResult::new("test2", TestStatus::Fail));

        report.add_suite(suite);
        report.calculate_pass_rate();

        assert_eq!(report.overall_pass_rate, 50.0);
    }

    #[test]
    fn test_result_report_to_text() {
        let report = ResultReport::new("run-1", "UOSC-0.1.0");
        let text = report.to_text();
        assert!(text.contains("UOSC Kernel Stress Test Report"));
        assert!(text.contains("run-1"));
    }

    #[test]
    fn test_result_report_to_json() {
        let report = ResultReport::new("run-1", "UOSC-0.1.0");
        let json = ReportGenerator::to_json(&report);
        assert!(json.is_ok());
    }
}
