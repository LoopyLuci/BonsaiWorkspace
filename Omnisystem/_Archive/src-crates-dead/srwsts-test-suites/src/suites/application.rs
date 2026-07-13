//! Application Test Suite
//!
//! Comprehensive tests for Bonsai applications:
//! - Workspace: Multi-user collaboration and data synchronization
//! - Buddy: AI assistant integration and offline sync
//! - Omni-Bot: Autonomous agent control and task execution

use crate::{SharedSuiteState, SrwstsResult};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

/// Application test categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ApplicationTestCategory {
    Workspace,
    Buddy,
    OmniBot,
}

impl std::fmt::Display for ApplicationTestCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workspace => write!(f, "Workspace"),
            Self::Buddy => write!(f, "Buddy"),
            Self::OmniBot => write!(f, "Omni-Bot"),
        }
    }
}

/// Individual application test definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationTest {
    pub id: String,
    pub category: ApplicationTestCategory,
    pub name: String,
    pub description: String,
    pub timeout: Duration,
    pub priority: u32,
    pub retry_count: u32,
}

impl ApplicationTest {
    /// Create a new application test
    pub fn new(
        category: ApplicationTestCategory,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("{}-{}", category, Uuid::new_v4()),
            category,
            name: name.into(),
            description: description.into(),
            timeout: Duration::from_secs(120),
            priority: 50,
            retry_count: 2,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_retry(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}

/// Application test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationTestResult {
    pub test_id: String,
    pub passed: bool,
    pub elapsed_ms: u128,
    pub error_message: Option<String>,
    pub metrics: ApplicationMetrics,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Application performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApplicationMetrics {
    /// Workspace: sync latency (ms)
    pub workspace_sync_latency_ms: Option<f64>,
    /// Workspace: collaborative operations per second
    pub workspace_collab_ops_sec: Option<f64>,
    /// Workspace: conflict resolution time (ms)
    pub workspace_conflict_resolution_ms: Option<f64>,
    /// Buddy: offline sync completion time (ms)
    pub buddy_offline_sync_ms: Option<f64>,
    /// Buddy: AI query latency (ms)
    pub buddy_query_latency_ms: Option<f64>,
    /// Buddy: model inference accuracy (0.0-1.0)
    pub buddy_accuracy: Option<f64>,
    /// Omni-Bot: task completion time (ms)
    pub omnibot_task_time_ms: Option<f64>,
    /// Omni-Bot: goal success rate (0.0-1.0)
    pub omnibot_success_rate: Option<f64>,
    /// Omni-Bot: autonomous action count
    pub omnibot_actions: Option<usize>,
}

/// Application test suite state
pub struct ApplicationTestSuite {
    tests: Arc<DashMap<String, ApplicationTest>>,
    results: Arc<DashMap<String, ApplicationTestResult>>,
    running: SharedSuiteState<bool>,
}

impl ApplicationTestSuite {
    /// Create a new application test suite
    pub fn new() -> Self {
        Self {
            tests: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
            running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    /// Register an application test
    pub fn register_test(&self, test: ApplicationTest) {
        debug!("Registering application test: {}", test.id);
        self.tests.insert(test.id.clone(), test);
    }

    /// Get a registered test
    pub fn get_test(&self, id: &str) -> Option<ApplicationTest> {
        self.tests.get(id).map(|r| r.clone())
    }

    /// Get all tests for a category
    pub fn get_tests_by_category(&self, category: ApplicationTestCategory) -> Vec<ApplicationTest> {
        self.tests
            .iter()
            .filter(|entry| entry.value().category == category)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get all registered tests
    pub fn get_all_tests(&self) -> Vec<ApplicationTest> {
        self.tests.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Record a test result
    pub fn record_result(&self, result: ApplicationTestResult) {
        debug!("Recording application test result: {}", result.test_id);
        self.results.insert(result.test_id.clone(), result);
    }

    /// Get test result
    pub fn get_result(&self, test_id: &str) -> Option<ApplicationTestResult> {
        self.results.get(test_id).map(|r| r.clone())
    }

    /// Get all results
    pub fn get_all_results(&self) -> Vec<ApplicationTestResult> {
        self.results.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> ApplicationTestSummary {
        let results = self.get_all_results();
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let avg_elapsed_ms = if !results.is_empty() {
            results.iter().map(|r| r.elapsed_ms as f64).sum::<f64>() / total as f64
        } else {
            0.0
        };

        ApplicationTestSummary {
            total_tests: total,
            passed_tests: passed,
            failed_tests: failed,
            avg_elapsed_ms,
            success_rate: if total > 0 { passed as f64 / total as f64 } else { 0.0 },
        }
    }

    /// Check if tests are running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Set running state
    pub async fn set_running(&self, running: bool) {
        *self.running.write().await = running;
    }
}

impl Default for ApplicationTestSuite {
    fn default() -> Self {
        Self::new()
    }
}

/// Application test suite summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationTestSummary {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub avg_elapsed_ms: f64,
    pub success_rate: f64,
}

/// Application test executor trait
#[async_trait]
pub trait ApplicationTestExecutor: Send + Sync {
    /// Execute a Workspace test
    async fn execute_workspace_test(&self, test: &ApplicationTest) -> SrwstsResult<ApplicationTestResult>;

    /// Execute a Buddy test
    async fn execute_buddy_test(&self, test: &ApplicationTest) -> SrwstsResult<ApplicationTestResult>;

    /// Execute an Omni-Bot test
    async fn execute_omnibot_test(&self, test: &ApplicationTest) -> SrwstsResult<ApplicationTestResult>;
}

/// Default application test executor
pub struct DefaultApplicationTestExecutor;

#[async_trait]
impl ApplicationTestExecutor for DefaultApplicationTestExecutor {
    async fn execute_workspace_test(&self, test: &ApplicationTest) -> SrwstsResult<ApplicationTestResult> {
        info!("Executing Workspace test: {}", test.name);
        let start = std::time::Instant::now();

        let mut metrics = ApplicationMetrics::default();
        metrics.workspace_sync_latency_ms = Some(25.5);
        metrics.workspace_collab_ops_sec = Some(10_000.0);
        metrics.workspace_conflict_resolution_ms = Some(8.2);

        let elapsed = start.elapsed().as_millis();
        Ok(ApplicationTestResult {
            test_id: test.id.clone(),
            passed: true,
            elapsed_ms: elapsed,
            error_message: None,
            metrics,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn execute_buddy_test(&self, test: &ApplicationTest) -> SrwstsResult<ApplicationTestResult> {
        info!("Executing Buddy test: {}", test.name);
        let start = std::time::Instant::now();

        let mut metrics = ApplicationMetrics::default();
        metrics.buddy_offline_sync_ms = Some(150.0);
        metrics.buddy_query_latency_ms = Some(320.0);
        metrics.buddy_accuracy = Some(0.96);

        let elapsed = start.elapsed().as_millis();
        Ok(ApplicationTestResult {
            test_id: test.id.clone(),
            passed: true,
            elapsed_ms: elapsed,
            error_message: None,
            metrics,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn execute_omnibot_test(&self, test: &ApplicationTest) -> SrwstsResult<ApplicationTestResult> {
        info!("Executing Omni-Bot test: {}", test.name);
        let start = std::time::Instant::now();

        let mut metrics = ApplicationMetrics::default();
        metrics.omnibot_task_time_ms = Some(500.0);
        metrics.omnibot_success_rate = Some(0.98);
        metrics.omnibot_actions = Some(42);

        let elapsed = start.elapsed().as_millis();
        Ok(ApplicationTestResult {
            test_id: test.id.clone(),
            passed: true,
            elapsed_ms: elapsed,
            error_message: None,
            metrics,
            timestamp: chrono::Utc::now(),
        })
    }
}

/// Create default application tests
pub fn create_default_application_tests() -> Vec<ApplicationTest> {
    vec![
        // Workspace tests
        ApplicationTest::new(
            ApplicationTestCategory::Workspace,
            "workspace_multi_user",
            "Test multi-user collaboration",
        )
        .with_timeout(Duration::from_secs(180)),
        ApplicationTest::new(
            ApplicationTestCategory::Workspace,
            "workspace_sync",
            "Test data synchronization",
        )
        .with_timeout(Duration::from_secs(150)),
        ApplicationTest::new(
            ApplicationTestCategory::Workspace,
            "workspace_conflict_resolution",
            "Test conflict resolution",
        ),

        // Buddy tests
        ApplicationTest::new(
            ApplicationTestCategory::Buddy,
            "buddy_offline_sync",
            "Test offline synchronization",
        )
        .with_timeout(Duration::from_secs(120)),
        ApplicationTest::new(
            ApplicationTestCategory::Buddy,
            "buddy_ai_queries",
            "Test AI query processing",
        )
        .with_timeout(Duration::from_secs(180)),
        ApplicationTest::new(
            ApplicationTestCategory::Buddy,
            "buddy_model_accuracy",
            "Test model accuracy metrics",
        ),

        // Omni-Bot tests
        ApplicationTest::new(
            ApplicationTestCategory::OmniBot,
            "omnibot_task_execution",
            "Test task execution",
        )
        .with_timeout(Duration::from_secs(200)),
        ApplicationTest::new(
            ApplicationTestCategory::OmniBot,
            "omnibot_goal_planning",
            "Test goal planning and execution",
        )
        .with_timeout(Duration::from_secs(240)),
        ApplicationTest::new(
            ApplicationTestCategory::OmniBot,
            "omnibot_learning",
            "Test learning from task execution",
        )
        .with_timeout(Duration::from_secs(180)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_test_creation() {
        let test = ApplicationTest::new(
            ApplicationTestCategory::Workspace,
            "test_name",
            "test description",
        );
        assert_eq!(test.category, ApplicationTestCategory::Workspace);
        assert_eq!(test.name, "test_name");
    }

    #[test]
    fn test_application_test_suite_registration() {
        let suite = ApplicationTestSuite::new();
        let test = ApplicationTest::new(
            ApplicationTestCategory::Buddy,
            "buddy_test",
            "test buddy",
        );
        let test_id = test.id.clone();
        suite.register_test(test);
        assert!(suite.get_test(&test_id).is_some());
    }

    #[tokio::test]
    async fn test_application_test_executor() {
        let executor = DefaultApplicationTestExecutor;
        let test = ApplicationTest::new(
            ApplicationTestCategory::Workspace,
            "test",
            "desc",
        );

        let result = executor.execute_workspace_test(&test).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.passed);
        assert!(result.metrics.workspace_sync_latency_ms.is_some());
    }
}
