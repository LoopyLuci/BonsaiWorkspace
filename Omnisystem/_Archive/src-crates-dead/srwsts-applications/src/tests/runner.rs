//! Test execution runner

use super::{
    BuddyStressTest, OmniBotStressTest, StressTest, TestContext, TestResult, TestStatus,
    WorkspaceStressTest,
};
use crate::errors::ApplicationStressResult;
use std::sync::Arc;
use tracing::{error, info};

/// Application test runner orchestrating all stress tests
pub struct ApplicationTestRunner {
    pub tests: Vec<Arc<dyn StressTest>>,
}

impl ApplicationTestRunner {
    /// Create a new test runner with default tests
    pub fn new() -> Self {
        let tests: Vec<Arc<dyn StressTest>> = vec![
            Arc::new(WorkspaceStressTest),
            Arc::new(BuddyStressTest),
            Arc::new(OmniBotStressTest),
        ];

        Self { tests }
    }

    /// Add a custom stress test
    pub fn add_test(&mut self, test: Arc<dyn StressTest>) {
        self.tests.push(test);
    }

    /// Run all tests with the given context
    pub async fn run_all(&self, ctx: &TestContext) -> ApplicationStressResult<Vec<TestResult>> {
        let mut results = Vec::new();

        for test in &self.tests {
            info!("Running test: {}", test.name());

            let result = match tokio::time::timeout(
                tokio::time::Duration::from_secs(ctx.timeout_secs),
                test.execute(ctx),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => {
                    error!("Test {} failed: {}", test.name(), e);
                    TestResult {
                        test_id: test.id().to_string(),
                        name: test.name().to_string(),
                        status: TestStatus::Failed,
                        duration_ms: 0,
                        error: Some(e.to_string()),
                        metrics: None,
                    }
                }
                Err(_) => {
                    error!("Test {} timed out", test.name());
                    TestResult {
                        test_id: test.id().to_string(),
                        name: test.name().to_string(),
                        status: TestStatus::Timeout,
                        duration_ms: ctx.timeout_secs * 1000,
                        error: Some("Test timeout".to_string()),
                        metrics: None,
                    }
                }
            };

            info!(
                "Test {} completed with status: {}",
                test.name(),
                result.status
            );

            results.push(result);
        }

        Ok(results)
    }

    /// Run a specific test by ID
    pub async fn run_test_by_id(
        &self,
        test_id: &str,
        ctx: &TestContext,
    ) -> ApplicationStressResult<Option<TestResult>> {
        for test in &self.tests {
            if test.id() == test_id {
                info!("Running test: {}", test.name());

                let result = match tokio::time::timeout(
                    tokio::time::Duration::from_secs(ctx.timeout_secs),
                    test.execute(ctx),
                )
                .await
                {
                    Ok(Ok(result)) => Some(result),
                    Ok(Err(e)) => {
                        error!("Test {} failed: {}", test.name(), e);
                        Some(TestResult {
                            test_id: test.id().to_string(),
                            name: test.name().to_string(),
                            status: TestStatus::Failed,
                            duration_ms: 0,
                            error: Some(e.to_string()),
                            metrics: None,
                        })
                    }
                    Err(_) => {
                        error!("Test {} timed out", test.name());
                        Some(TestResult {
                            test_id: test.id().to_string(),
                            name: test.name().to_string(),
                            status: TestStatus::Timeout,
                            duration_ms: ctx.timeout_secs * 1000,
                            error: Some("Test timeout".to_string()),
                            metrics: None,
                        })
                    }
                };

                return Ok(result);
            }
        }

        Ok(None)
    }

    /// Get test summary
    pub fn list_tests(&self) -> Vec<(String, String)> {
        self.tests
            .iter()
            .map(|t| (t.id().to_string(), t.name().to_string()))
            .collect()
    }

    /// Calculate aggregate results
    pub fn aggregate_results(results: &[TestResult]) -> AggregateResults {
        let total = results.len();
        let passed = results.iter().filter(|r| r.status == TestStatus::Passed).count();
        let failed = results.iter().filter(|r| r.status == TestStatus::Failed).count();
        let timeout = results.iter().filter(|r| r.status == TestStatus::Timeout).count();

        let total_duration_ms: u64 = results.iter().map(|r| r.duration_ms).sum();

        AggregateResults {
            total,
            passed,
            failed,
            timeout,
            total_duration_ms,
        }
    }
}

impl Default for ApplicationTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregate test results
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregateResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub timeout: usize,
    pub total_duration_ms: u64,
}

impl AggregateResults {
    /// Get pass rate
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total as f64) * 100.0
        }
    }

    /// Display summary
    pub fn summary(&self) -> String {
        format!(
            "Tests: {}/{} passed, {} failed, {} timeout | Total time: {:.2}s",
            self.passed,
            self.total,
            self.failed,
            self.timeout,
            self.total_duration_ms as f64 / 1000.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_creation() {
        let runner = ApplicationTestRunner::new();
        assert_eq!(runner.tests.len(), 3); // Default: Workspace, Buddy, OmniBot
    }

    #[test]
    fn test_list_tests() {
        let runner = ApplicationTestRunner::new();
        let tests = runner.list_tests();
        assert!(tests.len() >= 3);
        assert!(tests.iter().any(|t| t.0 == "workspace"));
        assert!(tests.iter().any(|t| t.0 == "buddy"));
        assert!(tests.iter().any(|t| t.0 == "omnibot"));
    }

    #[test]
    fn test_aggregate_results_pass_rate() {
        let results = vec![
            TestResult {
                test_id: "1".to_string(),
                name: "Test 1".to_string(),
                status: TestStatus::Passed,
                duration_ms: 100,
                error: None,
                metrics: None,
            },
            TestResult {
                test_id: "2".to_string(),
                name: "Test 2".to_string(),
                status: TestStatus::Failed,
                duration_ms: 100,
                error: None,
                metrics: None,
            },
        ];

        let agg = ApplicationTestRunner::aggregate_results(&results);
        assert_eq!(agg.total, 2);
        assert_eq!(agg.passed, 1);
        assert_eq!(agg.failed, 1);
        assert_eq!(agg.pass_rate(), 50.0);
    }

    #[test]
    fn test_aggregate_results_summary() {
        let results = vec![TestResult {
            test_id: "1".to_string(),
            name: "Test 1".to_string(),
            status: TestStatus::Passed,
            duration_ms: 1000,
            error: None,
            metrics: None,
        }];

        let agg = ApplicationTestRunner::aggregate_results(&results);
        let summary = agg.summary();
        assert!(summary.contains("Tests: 1/1 passed"));
    }
}
