//! Test Suite Executor
//!
//! Core execution engine for running test suites with parallelization and resource management.

use crate::{SrwstsResult, SuiteConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, RwLock};
use tracing::{debug, info, warn};

/// Execution context for a test suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub run_id: String,
    pub suite_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub config: ExecutorConfig,
}

/// Executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    pub max_concurrent: usize,
    pub test_timeout_seconds: u64,
    pub retry_count: u32,
    pub enable_profiling: bool,
    pub verbose: bool,
}

impl ExecutorConfig {
    /// Create from suite config
    pub fn from_suite_config(config: &SuiteConfig) -> Self {
        Self {
            max_concurrent: config.max_concurrent,
            test_timeout_seconds: config.test_timeout,
            retry_count: 1,
            enable_profiling: config.profile_performance,
            verbose: config.verbose,
        }
    }
}

/// Test execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionState {
    /// Awaiting execution
    Pending,
    /// Currently executing
    Running,
    /// Successfully completed
    Passed,
    /// Failed during execution
    Failed,
    /// Test was skipped
    Skipped,
    /// Execution was aborted
    Aborted,
}

impl std::fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Running => write!(f, "Running"),
            Self::Passed => write!(f, "Passed"),
            Self::Failed => write!(f, "Failed"),
            Self::Skipped => write!(f, "Skipped"),
            Self::Aborted => write!(f, "Aborted"),
        }
    }
}

/// Test execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_tests: usize,
    pub completed_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub total_elapsed_ms: u128,
    pub avg_test_time_ms: f64,
}

impl ExecutionStats {
    /// Update statistics after test completion
    pub fn record_test_completion(&mut self, elapsed_ms: u128, passed: bool) {
        self.completed_tests += 1;
        if passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }
        self.total_elapsed_ms += elapsed_ms;
        self.avg_test_time_ms = if self.completed_tests > 0 {
            self.total_elapsed_ms as f64 / self.completed_tests as f64
        } else {
            0.0
        };
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.passed_tests as f64 / self.total_tests as f64
        }
    }
}

/// Suite executor
pub struct SuiteExecutor {
    context: ExecutionContext,
    semaphore: Arc<Semaphore>,
    stats: Arc<RwLock<ExecutionStats>>,
    running: Arc<RwLock<bool>>,
}

impl SuiteExecutor {
    /// Create a new suite executor
    pub fn new(
        suite_id: impl Into<String>,
        config: ExecutorConfig,
    ) -> Self {
        let context = ExecutionContext {
            run_id: uuid::Uuid::new_v4().to_string(),
            suite_id: suite_id.into(),
            start_time: chrono::Utc::now(),
            config,
        };

        let max_concurrent = context.config.max_concurrent;

        Self {
            context,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get execution context
    pub fn context(&self) -> &ExecutionContext {
        &self.context
    }

    /// Start execution
    pub async fn start(&self) -> SrwstsResult<()> {
        *self.running.write().await = true;
        info!("Starting execution of suite: {}", self.context.suite_id);
        Ok(())
    }

    /// Stop execution
    pub async fn stop(&self) -> SrwstsResult<()> {
        *self.running.write().await = false;
        info!("Stopped execution of suite: {}", self.context.suite_id);
        Ok(())
    }

    /// Check if currently running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Acquire execution slot
    pub async fn acquire_slot(&self) {
        let _permit = self.semaphore.acquire().await;
        debug!("Acquired execution slot for {}", self.context.suite_id);
    }

    /// Record test completion
    pub async fn record_completion(&self, elapsed_ms: u128, passed: bool) {
        let mut stats = self.stats.write().await;
        stats.record_test_completion(elapsed_ms, passed);
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> ExecutionStats {
        self.stats.read().await.clone()
    }

    /// Set total test count
    pub async fn set_total_tests(&self, count: usize) {
        let mut stats = self.stats.write().await;
        stats.total_tests = count;
    }

    /// Execute a test with timeout
    pub async fn execute_with_timeout<F, T>(&self, future: F, timeout_seconds: u64) -> SrwstsResult<T>
    where
        F: std::future::Future<Output = SrwstsResult<T>>,
        T: Send + 'static,
    {
        let duration = Duration::from_secs(timeout_seconds);
        match tokio::time::timeout(duration, future).await {
            Ok(result) => result,
            Err(_) => {
                warn!("Test execution timeout ({} seconds)", timeout_seconds);
                Err(crate::SrwstsError::TestTimeout {
                    duration_secs: timeout_seconds,
                })
            }
        }
    }

    /// Get execution summary
    pub async fn summary(&self) -> ExecutionSummary {
        let stats = self.stats.read().await;
        let elapsed = chrono::Utc::now()
            .signed_duration_since(self.context.start_time)
            .num_milliseconds() as u128;

        ExecutionSummary {
            suite_id: self.context.suite_id.clone(),
            run_id: self.context.run_id.clone(),
            elapsed_ms: elapsed,
            total_tests: stats.total_tests,
            passed_tests: stats.passed_tests,
            failed_tests: stats.failed_tests,
            skipped_tests: stats.skipped_tests,
            success_rate: stats.success_rate(),
            avg_test_time_ms: stats.avg_test_time_ms,
        }
    }
}

/// Execution summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub suite_id: String,
    pub run_id: String,
    pub elapsed_ms: u128,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub success_rate: f64,
    pub avg_test_time_ms: f64,
}

impl ExecutionSummary {
    /// Format as human-readable string
    pub fn format_report(&self) -> String {
        format!(
            "Suite: {}\nRun ID: {}\nTotal Tests: {}\nPassed: {} ({:.1}%)\nFailed: {}\nSkipped: {}\nTotal Time: {:.2}s\nAvg Test Time: {:.2}ms",
            self.suite_id,
            self.run_id,
            self.total_tests,
            self.passed_tests,
            self.success_rate * 100.0,
            self.failed_tests,
            self.skipped_tests,
            self.elapsed_ms as f64 / 1000.0,
            self.avg_test_time_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_state_display() {
        assert_eq!(format!("{}", ExecutionState::Running), "Running");
        assert_eq!(format!("{}", ExecutionState::Passed), "Passed");
    }

    #[test]
    fn test_execution_stats() {
        let mut stats = ExecutionStats::default();
        stats.total_tests = 10;
        stats.record_test_completion(100, true);
        stats.record_test_completion(150, true);
        stats.record_test_completion(120, false);

        assert_eq!(stats.completed_tests, 3);
        assert_eq!(stats.passed_tests, 2);
        assert_eq!(stats.failed_tests, 1);
        assert_eq!(stats.success_rate(), 0.2); // 2/10
    }

    #[tokio::test]
    async fn test_suite_executor_creation() {
        let config = ExecutorConfig {
            max_concurrent: 4,
            test_timeout_seconds: 60,
            retry_count: 1,
            enable_profiling: false,
            verbose: false,
        };
        let executor = SuiteExecutor::new("test_suite", config);
        assert_eq!(executor.context().suite_id, "test_suite");
        assert!(!executor.is_running().await);
    }

    #[tokio::test]
    async fn test_suite_executor_lifecycle() {
        let config = ExecutorConfig {
            max_concurrent: 4,
            test_timeout_seconds: 60,
            retry_count: 1,
            enable_profiling: false,
            verbose: false,
        };
        let executor = SuiteExecutor::new("test_suite", config);

        executor.start().await.unwrap();
        assert!(executor.is_running().await);

        executor.stop().await.unwrap();
        assert!(!executor.is_running().await);
    }

    #[tokio::test]
    async fn test_execution_summary() {
        let config = ExecutorConfig {
            max_concurrent: 4,
            test_timeout_seconds: 60,
            retry_count: 1,
            enable_profiling: false,
            verbose: false,
        };
        let executor = SuiteExecutor::new("test_suite", config);
        executor.set_total_tests(5).await;
        executor.record_completion(100, true).await;

        let summary = executor.summary().await;
        assert_eq!(summary.total_tests, 5);
        assert_eq!(summary.passed_tests, 1);
    }
}
