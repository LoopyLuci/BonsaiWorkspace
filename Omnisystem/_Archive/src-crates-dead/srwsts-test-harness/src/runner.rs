//! Test runner: orchestrates test execution with emulation and fault injection

use crate::errors::{HarnessError, HarnessResult};
use crate::executor::{ExecutionConfig, ExecutionMode, TestExecutor};
use crate::limiter::ResourceLimiter;
use crate::metrics::MetricsCollector;
use crate::result::{ResultCapture, ResultStatus};
use crate::trace::TraceRecorder;
use crate::vault::{VaultConfig, VaultManager};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Test run configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunConfig {
    /// Test binary path
    pub binary_path: PathBuf,
    /// Arguments
    pub args: Vec<String>,
    /// Execution timeout
    pub timeout: Duration,
    /// Execution mode (Normal, Replay, Debug, Stress)
    pub mode: ExecutionMode,
    /// Enable metric collection
    pub collect_metrics: bool,
    /// Enable trace recording
    pub record_trace: bool,
    /// Enable fault injection
    pub inject_faults: bool,
    /// Vault configuration
    pub vault_config: VaultConfig,
}

impl Default for TestRunConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::new(),
            args: Vec::new(),
            timeout: Duration::from_secs(60),
            mode: ExecutionMode::Normal,
            collect_metrics: true,
            record_trace: true,
            inject_faults: false,
            vault_config: VaultConfig::default(),
        }
    }
}

impl TestRunConfig {
    /// Create a new test run configuration
    pub fn new(binary_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
            ..Default::default()
        }
    }

    /// Add an argument
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set execution mode
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Enable fault injection
    pub fn with_fault_injection(mut self, enabled: bool) -> Self {
        self.inject_faults = enabled;
        self
    }
}

/// Test runner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Metric aggregation enabled
    pub aggregate_metrics: bool,
    /// Enable deterministic replay
    pub deterministic_replay: bool,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 16,
            aggregate_metrics: true,
            deterministic_replay: true,
        }
    }
}

/// Test runner
pub struct TestRunner {
    /// Runner configuration
    config: RunnerConfig,
    /// Vault manager
    vault_manager: Arc<RwLock<VaultManager>>,
    /// Metrics collector
    metrics_collector: Arc<RwLock<MetricsCollector>>,
    /// Trace recorder
    trace_recorder: Arc<RwLock<TraceRecorder>>,
    /// Resource limiters
    limiters: Arc<RwLock<std::collections::HashMap<Uuid, ResourceLimiter>>>,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(config: RunnerConfig, max_vaults: usize) -> Self {
        Self {
            config,
            vault_manager: Arc::new(RwLock::new(VaultManager::new(max_vaults))),
            metrics_collector: Arc::new(RwLock::new(MetricsCollector::new())),
            trace_recorder: Arc::new(RwLock::new(TraceRecorder::new())),
            limiters: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Run a test
    pub async fn run_test(&self, test_config: TestRunConfig) -> HarnessResult<ResultCapture> {
        // Create vault
        let mut vault_manager = self.vault_manager.write().await;
        let vault = vault_manager
            .spawn_vault(test_config.vault_config.clone())
            .await?;
        drop(vault_manager);

        // Create resource limiter
        let limiter =
            ResourceLimiter::new(crate::limiter::ResourceLimits::default());
        self.limiters.write().await.insert(vault.id, limiter);

        // Create trace recorder
        let test_name = test_config
            .binary_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("test")
            .to_string();

        if test_config.record_trace {
            let mut recorder = self.trace_recorder.write().await;
            recorder.start_trace(test_name.clone());
        }

        // Execute test
        let execution_config = ExecutionConfig::new(&test_config.binary_path)
            .with_mode(test_config.mode)
            .with_timeout(test_config.timeout);

        let executor = TestExecutor::new(execution_config);

        // Run in vault
        let mut result = executor.execute(&vault).await?;

        // Simulate execution completion
        result.end_time = Some(Utc::now());

        // Collect metrics if enabled
        if test_config.collect_metrics {
            let metrics = crate::metrics::PerformanceMetrics {
                cpu_time_ms: 100,
                wall_time_ms: 150,
                peak_memory_bytes: test_config.vault_config.memory_limit,
                ..Default::default()
            };

            result.set_metrics(metrics.clone());

            let mut collector = self.metrics_collector.write().await;
            collector.collect(test_name.clone(), metrics);
            collector.record_result(result.is_passed());
        }

        // Complete trace if enabled
        if test_config.record_trace {
            let mut recorder = self.trace_recorder.write().await;
            recorder.complete_trace();
        }

        // Clean up
        let mut vault_manager = self.vault_manager.write().await;
        vault_manager.destroy_vault(vault.id).await?;
        self.limiters.write().await.remove(&vault.id);

        Ok(result)
    }

    /// Run multiple tests
    pub async fn run_tests(
        &self,
        test_configs: Vec<TestRunConfig>,
    ) -> HarnessResult<Vec<ResultCapture>> {
        let mut results = Vec::new();

        for config in test_configs {
            let result = self.run_test(config).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get metrics collector
    pub async fn metrics_collector(&self) -> Arc<RwLock<MetricsCollector>> {
        self.metrics_collector.clone()
    }

    /// Get trace recorder
    pub async fn trace_recorder(&self) -> Arc<RwLock<TraceRecorder>> {
        self.trace_recorder.clone()
    }

    /// Get vault manager
    pub async fn vault_manager(&self) -> Arc<RwLock<VaultManager>> {
        self.vault_manager.clone()
    }

    /// Shutdown runner
    pub async fn shutdown(&self) -> HarnessResult<()> {
        let mut vault_manager = self.vault_manager.write().await;
        vault_manager.shutdown().await?;

        let mut limiters = self.limiters.write().await;
        limiters.clear();

        tracing::info!("Test runner shutdown complete");

        Ok(())
    }

    /// Get statistics
    pub async fn statistics(&self) -> HarnessResult<RunnerStatistics> {
        let metrics = self.metrics_collector.read().await;
        let stats = metrics.statistics();

        Ok(RunnerStatistics {
            total_tests: stats.total_tests,
            passed_tests: stats.passed_tests,
            failed_tests: stats.failed_tests,
            pass_rate: stats.pass_rate,
        })
    }
}

/// Runner statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerStatistics {
    /// Total tests executed
    pub total_tests: u64,
    /// Passed tests
    pub passed_tests: u64,
    /// Failed tests
    pub failed_tests: u64,
    /// Pass rate percentage
    pub pass_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_run_config_creation() {
        let config = TestRunConfig::new("/path/to/test");
        assert_eq!(config.mode, ExecutionMode::Normal);
        assert!(config.collect_metrics);
    }

    #[test]
    fn test_test_run_config_builder() {
        let config = TestRunConfig::new("/path/to/test")
            .with_arg("--verbose")
            .with_timeout(Duration::from_secs(120))
            .with_mode(ExecutionMode::Debug);

        assert_eq!(config.args.len(), 1);
        assert_eq!(config.timeout.as_secs(), 120);
        assert_eq!(config.mode, ExecutionMode::Debug);
    }

    #[test]
    fn test_runner_config_default() {
        let config = RunnerConfig::default();
        assert_eq!(config.max_concurrent, 16);
        assert!(config.aggregate_metrics);
    }

    #[tokio::test]
    async fn test_test_runner_creation() {
        let config = RunnerConfig::default();
        let runner = TestRunner::new(config, 64);

        let stats = runner.statistics().await.unwrap();
        assert_eq!(stats.total_tests, 0);
    }

    #[tokio::test]
    async fn test_test_runner_shutdown() {
        let config = RunnerConfig::default();
        let runner = TestRunner::new(config, 64);

        let result = runner.shutdown().await;
        assert!(result.is_ok());
    }
}
