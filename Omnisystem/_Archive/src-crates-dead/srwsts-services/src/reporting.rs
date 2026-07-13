//! Test reporting and result analysis
//!
//! Generate comprehensive reports with:
//! - Service health metrics
//! - Performance statistics
//! - Failure recovery times
//! - Dependency impact analysis

use crate::types::TestResult;
use crate::ServiceMetrics;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Individual service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthStatus {
    pub service_id: String,
    pub service_type: String,
    pub is_healthy: bool,
    pub uptime_percent: f64,
    pub error_rate: f64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
}

impl ServiceHealthStatus {
    pub fn from_metrics(service_id: impl Into<String>, service_type: impl Into<String>, metrics: &ServiceMetrics) -> Self {
        Self {
            service_id: service_id.into(),
            service_type: service_type.into(),
            is_healthy: metrics.success_rate() > 95.0,
            uptime_percent: metrics.success_rate(),
            error_rate: metrics.errors.error_rate,
            avg_latency_ms: metrics.latency.mean_ms,
            p99_latency_ms: metrics.latency.p99_ms,
        }
    }
}

/// Performance analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysis {
    pub total_operations: u64,
    pub operations_per_second: f64,
    pub avg_throughput_mbps: f64,
    pub peak_latency_ms: f64,
    pub tail_latency_p99_ms: f64,
    pub error_count: u64,
    pub error_rate_percent: f64,
}

impl PerformanceAnalysis {
    pub fn from_metrics(metrics: &ServiceMetrics, elapsed_secs: f64) -> Self {
        Self {
            total_operations: metrics.total_operations,
            operations_per_second: metrics.total_operations as f64 / elapsed_secs.max(1.0),
            avg_throughput_mbps: if metrics.throughput.total_bytes > 0 {
                (metrics.throughput.total_bytes as f64 / elapsed_secs.max(1.0)) / 1_000_000.0
            } else {
                0.0
            },
            peak_latency_ms: metrics.latency.max_ms,
            tail_latency_p99_ms: metrics.latency.p99_ms,
            error_count: metrics.errors.total_errors,
            error_rate_percent: metrics.errors.error_rate,
        }
    }
}

/// Failure recovery analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecoveryAnalysis {
    pub total_failures: usize,
    pub recovered_failures: usize,
    pub unrecovered_failures: usize,
    pub avg_recovery_time_ms: f64,
    pub longest_recovery_time_ms: f64,
    pub recovery_success_rate: f64,
}

impl FailureRecoveryAnalysis {
    pub fn new() -> Self {
        Self {
            total_failures: 0,
            recovered_failures: 0,
            unrecovered_failures: 0,
            avg_recovery_time_ms: 0.0,
            longest_recovery_time_ms: 0.0,
            recovery_success_rate: 0.0,
        }
    }

    pub fn calculate(&mut self) {
        if self.total_failures > 0 {
            self.recovery_success_rate =
                (self.recovered_failures as f64 / self.total_failures as f64) * 100.0;
        }
    }
}

/// Dependency impact report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyImpactReport {
    pub affected_services: Vec<String>,
    pub impact_severity: String, // Low, Medium, High, Critical
    pub cascading_failures: usize,
    pub estimated_recovery_time_ms: u64,
}

impl DependencyImpactReport {
    pub fn new() -> Self {
        Self {
            affected_services: Vec::new(),
            impact_severity: "Low".to_string(),
            cascading_failures: 0,
            estimated_recovery_time_ms: 0,
        }
    }
}

/// Comprehensive test report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReport {
    pub report_id: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub success_rate: f64,
    pub test_results: Vec<TestResult>,
    pub service_metrics: HashMap<String, ServiceMetrics>,
    pub service_health: Vec<ServiceHealthStatus>,
    pub performance_analysis: HashMap<String, PerformanceAnalysis>,
    pub failure_recovery: FailureRecoveryAnalysis,
    pub dependency_impact: Vec<DependencyImpactReport>,
    pub summary: String,
    pub recommendations: Vec<String>,
}

impl TestReport {
    pub fn new() -> Self {
        Self {
            report_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            duration_ms: 0,
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            skipped_tests: 0,
            success_rate: 0.0,
            test_results: Vec::new(),
            service_metrics: HashMap::new(),
            service_health: Vec::new(),
            performance_analysis: HashMap::new(),
            failure_recovery: FailureRecoveryAnalysis::new(),
            dependency_impact: Vec::new(),
            summary: String::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.success_rate >= 95.0
    }

    /// Generate service health report
    pub fn generate_service_health(&mut self) {
        for (service_id, metrics) in &self.service_metrics {
            let health = ServiceHealthStatus::from_metrics(
                service_id,
                "unknown",
                metrics,
            );
            self.service_health.push(health);
        }
    }

    /// Generate performance analysis
    pub fn generate_performance_analysis(&mut self, elapsed_secs: f64) {
        for (service_id, metrics) in &self.service_metrics {
            let analysis = PerformanceAnalysis::from_metrics(metrics, elapsed_secs);
            self.performance_analysis
                .insert(service_id.clone(), analysis);
        }
    }

    /// Generate summary
    pub fn generate_summary(&mut self) {
        let mut summary = String::new();
        summary.push_str(&format!("Test Run Summary\n"));
        summary.push_str(&format!("================\n"));
        summary.push_str(&format!("Report ID: {}\n", self.report_id));
        summary.push_str(&format!("Timestamp: {}\n", self.timestamp));
        summary.push_str(&format!("Total Tests: {}\n", self.total_tests));
        summary.push_str(&format!("Passed: {}\n", self.passed_tests));
        summary.push_str(&format!("Failed: {}\n", self.failed_tests));
        summary.push_str(&format!("Success Rate: {:.1}%\n", self.success_rate));

        if !self.service_metrics.is_empty() {
            summary.push_str(&format!("\nService Metrics\n"));
            summary.push_str(&format!("---------------\n"));
            for (service_id, metrics) in &self.service_metrics {
                summary.push_str(&format!(
                    "{}: {} ops, {:.1}% success, p99: {:.1}ms\n",
                    service_id,
                    metrics.total_operations,
                    metrics.success_rate(),
                    metrics.latency.p99_ms
                ));
            }
        }

        self.summary = summary;
    }

    /// Add recommendation
    pub fn add_recommendation(&mut self, recommendation: impl Into<String>) {
        self.recommendations.push(recommendation.into());
    }

    /// Export as JSON string
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&self)
    }

    /// Export as JSON with indentation
    pub fn to_json_formatted(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&self)
    }
}

impl Default for TestReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_creation() {
        let report = TestReport::new();
        assert!(!report.report_id.is_empty());
        assert_eq!(report.total_tests, 0);
    }

    #[test]
    fn test_report_success_check() {
        let mut report = TestReport::new();
        report.success_rate = 96.0;
        assert!(report.is_success());
        report.success_rate = 94.0;
        assert!(!report.is_success());
    }

    #[test]
    fn test_report_to_json() {
        let report = TestReport::new();
        let json = report.to_json().unwrap();
        assert!(json.contains("report_id"));
        assert!(json.contains("timestamp"));
    }

    #[test]
    fn test_service_health_status() {
        let health = ServiceHealthStatus {
            service_id: "test".to_string(),
            service_type: "p2p".to_string(),
            is_healthy: true,
            uptime_percent: 99.5,
            error_rate: 0.5,
            avg_latency_ms: 10.0,
            p99_latency_ms: 25.0,
        };
        assert!(health.is_healthy);
        assert!(health.error_rate < 1.0);
    }

    #[test]
    fn test_performance_analysis() {
        let analysis = PerformanceAnalysis {
            total_operations: 10000,
            operations_per_second: 1000.0,
            avg_throughput_mbps: 100.0,
            peak_latency_ms: 50.0,
            tail_latency_p99_ms: 30.0,
            error_count: 10,
            error_rate_percent: 0.1,
        };
        assert_eq!(analysis.total_operations, 10000);
        assert_eq!(analysis.operations_per_second, 1000.0);
    }

    #[test]
    fn test_report_summary_generation() {
        let mut report = TestReport::new();
        report.total_tests = 100;
        report.passed_tests = 95;
        report.failed_tests = 5;
        report.success_rate = 95.0;
        report.generate_summary();
        assert!(!report.summary.is_empty());
        assert!(report.summary.contains("100"));
    }
}
