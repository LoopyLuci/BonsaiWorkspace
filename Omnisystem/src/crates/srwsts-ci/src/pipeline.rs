//! Multi-tier CI pipeline: smoke tests, full suite, nightly validation

use crate::artifacts::ArtifactCollector;
use crate::detection::RegressionDetector;
use crate::errors::{CIError, CIResult};
use crate::metrics::PerformanceMetrics;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

/// Pipeline stage type
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    SmokeTests,      // Critical path, blocks merge
    FullSuite,       // Comprehensive, runs after smoke
    Nightly,         // Deep validation, scheduled
    BugDiscovery,    // Optional AI-assisted testing
}

/// CI pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub enable_smoke_tests: bool,
    pub enable_full_suite: bool,
    pub enable_nightly: bool,
    pub smoke_test_timeout: Duration,
    pub full_suite_timeout: Duration,
    pub block_on_regression: bool,
    pub auto_report: bool,
    pub enable_ai_advisor: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enable_smoke_tests: true,
            enable_full_suite: true,
            enable_nightly: false,
            smoke_test_timeout: Duration::from_secs(300),
            full_suite_timeout: Duration::from_secs(3600),
            block_on_regression: true,
            auto_report: true,
            enable_ai_advisor: false,
        }
    }
}

/// Pipeline execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub stage: PipelineStage,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub status: PipelineStatus,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub regression_count: usize,
    pub metrics: Option<PerformanceMetrics>,
    pub error_message: Option<String>,
}

/// Pipeline execution status
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum PipelineStatus {
    Passed,
    Failed,
    Blocked,     // Regression detected, merge blocked
    Timeout,
    Skipped,
}

/// Trait for test runner implementations
#[async_trait]
pub trait TestRunner: Send + Sync {
    async fn run_smoke_tests(&self) -> CIResult<(usize, usize, Option<PerformanceMetrics>)>;
    async fn run_full_suite(&self) -> CIResult<(usize, usize, Option<PerformanceMetrics>)>;
    async fn run_nightly(&self) -> CIResult<(usize, usize, Option<PerformanceMetrics>)>;
}

/// Mock test runner for testing
pub struct MockTestRunner {
    pass_rate: f64,
}

impl MockTestRunner {
    pub fn new(pass_rate: f64) -> Self {
        Self { pass_rate }
    }
}

#[async_trait]
impl TestRunner for MockTestRunner {
    async fn run_smoke_tests(&self) -> CIResult<(usize, usize, Option<PerformanceMetrics>)> {
        let total = 10;
        let passed = (total as f64 * self.pass_rate) as usize;
        let failed = total - passed;
        Ok((passed, failed, None))
    }

    async fn run_full_suite(&self) -> CIResult<(usize, usize, Option<PerformanceMetrics>)> {
        let total = 100;
        let passed = (total as f64 * self.pass_rate) as usize;
        let failed = total - passed;
        Ok((passed, failed, None))
    }

    async fn run_nightly(&self) -> CIResult<(usize, usize, Option<PerformanceMetrics>)> {
        let total = 500;
        let passed = (total as f64 * self.pass_rate) as usize;
        let failed = total - passed;
        Ok((passed, failed, None))
    }
}

/// CI Pipeline orchestrator
pub struct CIPipeline {
    config: PipelineConfig,
    test_runner: std::sync::Arc<dyn TestRunner>,
    regression_detector: RegressionDetector,
    artifact_collector: Option<ArtifactCollector>,
    results: dashmap::DashMap<PipelineStage, PipelineResult>,
}

impl CIPipeline {
    /// Create new pipeline
    pub fn new(config: PipelineConfig, test_runner: std::sync::Arc<dyn TestRunner>) -> Self {
        Self {
            config,
            test_runner,
            regression_detector: RegressionDetector::new(),
            artifact_collector: None,
            results: dashmap::DashMap::new(),
        }
    }

    /// Enable artifact collection
    pub fn with_artifacts(mut self, collector: ArtifactCollector) -> Self {
        self.artifact_collector = Some(collector);
        self
    }

    /// Run smoke tests (critical path)
    pub async fn run_smoke_tests(&self) -> CIResult<PipelineResult> {
        info!("Starting smoke tests");
        let started_at = Utc::now();

        match tokio::time::timeout(
            self.config.smoke_test_timeout,
            self.test_runner.run_smoke_tests(),
        )
        .await
        {
            Ok(Ok((passed, failed, metrics))) => {
                let status = if failed == 0 {
                    PipelineStatus::Passed
                } else if self.config.block_on_regression {
                    PipelineStatus::Blocked
                } else {
                    PipelineStatus::Failed
                };

                info!(
                    "Smoke tests completed: {} passed, {} failed",
                    passed, failed
                );

                let result = PipelineResult {
                    stage: PipelineStage::SmokeTests,
                    started_at,
                    completed_at: Utc::now(),
                    status,
                    passed_tests: passed,
                    failed_tests: failed,
                    regression_count: failed,
                    metrics,
                    error_message: None,
                };

                self.results
                    .insert(PipelineStage::SmokeTests, result.clone());
                Ok(result)
            }
            Ok(Err(e)) => {
                error!("Smoke tests failed: {}", e);
                let result = PipelineResult {
                    stage: PipelineStage::SmokeTests,
                    started_at,
                    completed_at: Utc::now(),
                    status: PipelineStatus::Failed,
                    passed_tests: 0,
                    failed_tests: 0,
                    regression_count: 0,
                    metrics: None,
                    error_message: Some(e.to_string()),
                };
                self.results
                    .insert(PipelineStage::SmokeTests, result.clone());
                Err(e)
            }
            Err(_) => {
                error!("Smoke tests timed out");
                let result = PipelineResult {
                    stage: PipelineStage::SmokeTests,
                    started_at,
                    completed_at: Utc::now(),
                    status: PipelineStatus::Timeout,
                    passed_tests: 0,
                    failed_tests: 0,
                    regression_count: 0,
                    metrics: None,
                    error_message: Some("Smoke tests timeout".to_string()),
                };
                self.results
                    .insert(PipelineStage::SmokeTests, result.clone());
                Err(CIError::TestExecutionFailed(
                    "Smoke tests timeout".to_string(),
                ))
            }
        }
    }

    /// Run full test suite
    pub async fn run_full_suite(&self) -> CIResult<PipelineResult> {
        info!("Starting full test suite");
        let started_at = Utc::now();

        // Check smoke tests passed first
        if let Some(smoke_result) = self.results.get(&PipelineStage::SmokeTests) {
            if smoke_result.status != PipelineStatus::Passed {
                info!("Skipping full suite due to failed smoke tests");
                let result = PipelineResult {
                    stage: PipelineStage::FullSuite,
                    started_at,
                    completed_at: Utc::now(),
                    status: PipelineStatus::Skipped,
                    passed_tests: 0,
                    failed_tests: 0,
                    regression_count: 0,
                    metrics: None,
                    error_message: None,
                };
                self.results.insert(PipelineStage::FullSuite, result.clone());
                return Ok(result);
            }
        }

        match tokio::time::timeout(
            self.config.full_suite_timeout,
            self.test_runner.run_full_suite(),
        )
        .await
        {
            Ok(Ok((passed, failed, metrics))) => {
                let status = if failed == 0 {
                    PipelineStatus::Passed
                } else {
                    PipelineStatus::Failed
                };

                info!("Full suite completed: {} passed, {} failed", passed, failed);

                let result = PipelineResult {
                    stage: PipelineStage::FullSuite,
                    started_at,
                    completed_at: Utc::now(),
                    status,
                    passed_tests: passed,
                    failed_tests: failed,
                    regression_count: failed,
                    metrics,
                    error_message: None,
                };

                self.results.insert(PipelineStage::FullSuite, result.clone());
                Ok(result)
            }
            Ok(Err(e)) => {
                error!("Full suite failed: {}", e);
                let result = PipelineResult {
                    stage: PipelineStage::FullSuite,
                    started_at,
                    completed_at: Utc::now(),
                    status: PipelineStatus::Failed,
                    passed_tests: 0,
                    failed_tests: 0,
                    regression_count: 0,
                    metrics: None,
                    error_message: Some(e.to_string()),
                };
                self.results.insert(PipelineStage::FullSuite, result.clone());
                Err(e)
            }
            Err(_) => {
                error!("Full suite timed out");
                let result = PipelineResult {
                    stage: PipelineStage::FullSuite,
                    started_at,
                    completed_at: Utc::now(),
                    status: PipelineStatus::Timeout,
                    passed_tests: 0,
                    failed_tests: 0,
                    regression_count: 0,
                    metrics: None,
                    error_message: Some("Full suite timeout".to_string()),
                };
                self.results.insert(PipelineStage::FullSuite, result.clone());
                Err(CIError::TestExecutionFailed("Full suite timeout".to_string()))
            }
        }
    }

    /// Run nightly suite
    pub async fn run_nightly(&self) -> CIResult<PipelineResult> {
        info!("Starting nightly test suite");
        let started_at = Utc::now();

        match tokio::time::timeout(
            Duration::from_secs(86400), // 24 hour timeout
            self.test_runner.run_nightly(),
        )
        .await
        {
            Ok(Ok((passed, failed, metrics))) => {
                let status = if failed == 0 {
                    PipelineStatus::Passed
                } else {
                    PipelineStatus::Failed
                };

                info!("Nightly suite completed: {} passed, {} failed", passed, failed);

                let result = PipelineResult {
                    stage: PipelineStage::Nightly,
                    started_at,
                    completed_at: Utc::now(),
                    status,
                    passed_tests: passed,
                    failed_tests: failed,
                    regression_count: failed,
                    metrics,
                    error_message: None,
                };

                self.results.insert(PipelineStage::Nightly, result.clone());
                Ok(result)
            }
            Ok(Err(e)) => {
                error!("Nightly suite failed: {}", e);
                let result = PipelineResult {
                    stage: PipelineStage::Nightly,
                    started_at,
                    completed_at: Utc::now(),
                    status: PipelineStatus::Failed,
                    passed_tests: 0,
                    failed_tests: 0,
                    regression_count: 0,
                    metrics: None,
                    error_message: Some(e.to_string()),
                };
                self.results.insert(PipelineStage::Nightly, result.clone());
                Err(e)
            }
            Err(_) => {
                error!("Nightly suite timed out");
                let result = PipelineResult {
                    stage: PipelineStage::Nightly,
                    started_at,
                    completed_at: Utc::now(),
                    status: PipelineStatus::Timeout,
                    passed_tests: 0,
                    failed_tests: 0,
                    regression_count: 0,
                    metrics: None,
                    error_message: Some("Nightly suite timeout".to_string()),
                };
                self.results.insert(PipelineStage::Nightly, result.clone());
                Err(CIError::TestExecutionFailed("Nightly suite timeout".to_string()))
            }
        }
    }

    /// Get result for stage
    pub fn get_result(&self, stage: &PipelineStage) -> Option<PipelineResult> {
        self.results.get(stage).map(|r| r.clone())
    }

    /// Get all results
    pub fn get_all_results(&self) -> Vec<PipelineResult> {
        self.results
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Check if merge should be blocked
    pub fn should_block_merge(&self) -> bool {
        if let Some(smoke) = self.get_result(&PipelineStage::SmokeTests) {
            smoke.status == PipelineStatus::Blocked || smoke.status == PipelineStatus::Failed
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_smoke_tests_passed() {
        let config = PipelineConfig::default();
        let runner = std::sync::Arc::new(MockTestRunner::new(1.0));
        let pipeline = CIPipeline::new(config, runner);

        let result = pipeline.run_smoke_tests().await.unwrap();
        assert_eq!(result.status, PipelineStatus::Passed);
        assert_eq!(result.failed_tests, 0);
    }

    #[tokio::test]
    async fn test_smoke_tests_failed() {
        let config = PipelineConfig::default();
        let runner = std::sync::Arc::new(MockTestRunner::new(0.5));
        let pipeline = CIPipeline::new(config, runner);

        let result = pipeline.run_smoke_tests().await.unwrap();
        assert_eq!(result.status, PipelineStatus::Blocked);
        assert!(result.failed_tests > 0);
    }

    #[tokio::test]
    async fn test_full_suite_skipped_on_smoke_failure() {
        let config = PipelineConfig::default();
        let runner = std::sync::Arc::new(MockTestRunner::new(0.5));
        let pipeline = CIPipeline::new(config, runner);

        let _ = pipeline.run_smoke_tests().await;
        let result = pipeline.run_full_suite().await.unwrap();

        assert_eq!(result.status, PipelineStatus::Skipped);
    }

    #[tokio::test]
    async fn test_should_block_merge_on_regression() {
        let config = PipelineConfig::default();
        let runner = std::sync::Arc::new(MockTestRunner::new(0.5));
        let pipeline = CIPipeline::new(config, runner);

        let _ = pipeline.run_smoke_tests().await;
        assert!(pipeline.should_block_merge());
    }

    #[tokio::test]
    async fn test_pipeline_result_serialization() {
        let result = PipelineResult {
            stage: PipelineStage::SmokeTests,
            started_at: Utc::now(),
            completed_at: Utc::now(),
            status: PipelineStatus::Passed,
            passed_tests: 10,
            failed_tests: 0,
            regression_count: 0,
            metrics: None,
            error_message: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PipelineResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.stage, deserialized.stage);
        assert_eq!(result.status, deserialized.status);
    }

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert!(config.enable_smoke_tests);
        assert!(config.enable_full_suite);
        assert!(!config.enable_nightly);
        assert!(config.block_on_regression);
    }

    #[tokio::test]
    async fn test_nightly_suite() {
        let config = PipelineConfig::default();
        let runner = std::sync::Arc::new(MockTestRunner::new(1.0));
        let pipeline = CIPipeline::new(config, runner);

        let result = pipeline.run_nightly().await.unwrap();
        assert_eq!(result.stage, PipelineStage::Nightly);
        assert_eq!(result.status, PipelineStatus::Passed);
    }

    #[tokio::test]
    async fn test_get_all_results() {
        let config = PipelineConfig::default();
        let runner = std::sync::Arc::new(MockTestRunner::new(1.0));
        let pipeline = CIPipeline::new(config, runner);

        let _ = pipeline.run_smoke_tests().await;
        let _ = pipeline.run_full_suite().await;

        let all_results = pipeline.get_all_results();
        assert!(all_results.len() >= 2);
    }
}
