//! Test Suite Coordinator
//!
//! Orchestrates execution of multiple test suites with coordination and reporting.

use crate::{
    SrwstsResult, SuiteConfig, registry::TestSuiteRegistry, executor::{SuiteExecutor, ExecutorConfig}
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Test suite coordinator
pub struct SuiteCoordinator {
    config: SuiteConfig,
    registry: Arc<TestSuiteRegistry>,
    executors: Arc<DashMap<String, SuiteExecutor>>,
    running: Arc<RwLock<bool>>,
}

impl SuiteCoordinator {
    /// Create a new suite coordinator
    pub fn new(config: SuiteConfig, registry: Arc<TestSuiteRegistry>) -> Self {
        Self {
            config,
            registry,
            executors: Arc::new(DashMap::new()),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Register a suite executor
    pub fn register_executor(&self, suite_id: String, executor: SuiteExecutor) -> SrwstsResult<()> {
        debug!("Registering executor for suite: {}", suite_id);
        self.executors.insert(suite_id, executor);
        Ok(())
    }

    /// Get an executor by suite ID
    pub fn get_executor(&self, suite_id: &str) -> Option<SuiteExecutor> {
        self.executors.get(suite_id).map(|entry| {
            // Reconstruct the executor (this is a workaround since we can't clone executors)
            let original = entry.value();
            SuiteExecutor::new(
                suite_id,
                ExecutorConfig {
                    max_concurrent: original.context().config.max_concurrent,
                    test_timeout_seconds: original.context().config.test_timeout_seconds,
                    retry_count: original.context().config.retry_count,
                    enable_profiling: original.context().config.enable_profiling,
                    verbose: original.context().config.verbose,
                },
            )
        })
    }

    /// Start all registered executors
    pub async fn start_all(&self) -> SrwstsResult<()> {
        info!("Starting all test suite executors");
        *self.running.write().await = true;

        for entry in self.executors.iter() {
            entry.value().start().await?;
        }

        Ok(())
    }

    /// Stop all registered executors
    pub async fn stop_all(&self) -> SrwstsResult<()> {
        info!("Stopping all test suite executors");
        *self.running.write().await = false;

        for entry in self.executors.iter() {
            entry.value().stop().await?;
        }

        Ok(())
    }

    /// Check if coordinator is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get executor count
    pub fn executor_count(&self) -> usize {
        self.executors.len()
    }

    /// Get all executors
    pub fn get_all_executors(&self) -> Vec<(String, SuiteExecutor)> {
        self.executors
            .iter()
            .map(|entry| {
                let suite_id = entry.key().clone();
                let original = entry.value();
                let executor = SuiteExecutor::new(
                    suite_id.clone(),
                    ExecutorConfig {
                        max_concurrent: original.context().config.max_concurrent,
                        test_timeout_seconds: original.context().config.test_timeout_seconds,
                        retry_count: original.context().config.retry_count,
                        enable_profiling: original.context().config.enable_profiling,
                        verbose: original.context().config.verbose,
                    },
                );
                (suite_id, executor)
            })
            .collect()
    }

    /// Generate coordination report
    pub async fn generate_report(&self) -> CoordinationReport {
        let executors = self.executor_count();
        let running = self.is_running().await;

        let mut suite_reports = Vec::new();
        for entry in self.executors.iter() {
            let summary = entry.value().summary().await;
            suite_reports.push(SuiteExecutionReport {
                suite_id: summary.suite_id,
                run_id: summary.run_id,
                passed: summary.passed_tests,
                failed: summary.failed_tests,
                total: summary.total_tests,
                success_rate: summary.success_rate,
            });
        }

        let total_passed: usize = suite_reports.iter().map(|r| r.passed).sum();
        let total_failed: usize = suite_reports.iter().map(|r| r.failed).sum();
        let total_tests: usize = suite_reports.iter().map(|r| r.total).sum();

        CoordinationReport {
            timestamp: chrono::Utc::now(),
            total_executors: executors,
            running,
            total_tests,
            total_passed,
            total_failed,
            suite_reports,
        }
    }
}

/// Per-suite execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteExecutionReport {
    pub suite_id: String,
    pub run_id: String,
    pub passed: usize,
    pub failed: usize,
    pub total: usize,
    pub success_rate: f64,
}

/// Coordination report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_executors: usize,
    pub running: bool,
    pub total_tests: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub suite_reports: Vec<SuiteExecutionReport>,
}

impl CoordinationReport {
    /// Format as human-readable report
    pub fn format_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== Test Coordination Report ===\n");
        report.push_str(&format!("Timestamp: {}\n", self.timestamp));
        report.push_str(&format!("Status: {}\n", if self.running { "Running" } else { "Stopped" }));
        report.push_str(&format!("Total Executors: {}\n", self.total_executors));
        report.push_str(&format!("\nOverall Results:\n"));
        report.push_str(&format!("  Total Tests: {}\n", self.total_tests));
        report.push_str(&format!("  Passed: {}\n", self.total_passed));
        report.push_str(&format!("  Failed: {}\n", self.total_failed));

        let overall_rate = if self.total_tests > 0 {
            self.total_passed as f64 / self.total_tests as f64 * 100.0
        } else {
            0.0
        };
        report.push_str(&format!("  Success Rate: {:.1}%\n", overall_rate));

        report.push_str(&format!("\nSuite Reports:\n"));
        for suite in &self.suite_reports {
            report.push_str(&format!("  {} [{}]\n", suite.suite_id, suite.run_id));
            report.push_str(&format!("    Passed: {} / {}\n", suite.passed, suite.total));
            report.push_str(&format!("    Success Rate: {:.1}%\n", suite.success_rate * 100.0));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordination_report_creation() {
        let report = CoordinationReport {
            timestamp: chrono::Utc::now(),
            total_executors: 6,
            running: false,
            total_tests: 100,
            total_passed: 95,
            total_failed: 5,
            suite_reports: vec![],
        };

        assert_eq!(report.total_executors, 6);
        assert_eq!(report.total_tests, 100);
    }

    #[tokio::test]
    async fn test_coordinator_creation() {
        let config = SuiteConfig::default();
        let registry = Arc::new(TestSuiteRegistry::new());
        let coordinator = SuiteCoordinator::new(config, registry);

        assert_eq!(coordinator.executor_count(), 0);
        assert!(!coordinator.is_running().await);
    }

    #[test]
    fn test_suite_execution_report() {
        let report = SuiteExecutionReport {
            suite_id: "test_suite".to_string(),
            run_id: "run_1".to_string(),
            passed: 9,
            failed: 1,
            total: 10,
            success_rate: 0.9,
        };

        assert_eq!(report.passed, 9);
        assert_eq!(report.success_rate, 0.9);
    }
}
