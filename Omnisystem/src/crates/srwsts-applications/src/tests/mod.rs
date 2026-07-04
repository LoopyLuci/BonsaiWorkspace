//! Application stress test implementations

mod context;
mod workspace_tests;
mod buddy_tests;
mod omnibot_tests;
mod runner;

pub use context::TestContext;
pub use workspace_tests::WorkspaceStressTest;
pub use buddy_tests::BuddyStressTest;
pub use omnibot_tests::OmniBotStressTest;
pub use runner::ApplicationTestRunner;

use crate::errors::ApplicationStressResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub metrics: Option<String>,
}

/// Test execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Timeout,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "PASSED"),
            TestStatus::Failed => write!(f, "FAILED"),
            TestStatus::Skipped => write!(f, "SKIPPED"),
            TestStatus::Timeout => write!(f, "TIMEOUT"),
        }
    }
}

/// Trait for stress test implementations
#[async_trait]
pub trait StressTest: Send + Sync {
    /// Get test ID
    fn id(&self) -> &str;

    /// Get test name
    fn name(&self) -> &str;

    /// Execute the stress test
    async fn execute(&self, ctx: &TestContext) -> ApplicationStressResult<TestResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_display() {
        assert_eq!(TestStatus::Passed.to_string(), "PASSED");
        assert_eq!(TestStatus::Failed.to_string(), "FAILED");
        assert_eq!(TestStatus::Timeout.to_string(), "TIMEOUT");
    }

    #[test]
    fn test_result_creation() {
        let result = TestResult {
            test_id: "test-01".to_string(),
            name: "Test 01".to_string(),
            status: TestStatus::Passed,
            duration_ms: 1000,
            error: None,
            metrics: None,
        };

        assert_eq!(result.status, TestStatus::Passed);
        assert_eq!(result.duration_ms, 1000);
    }
}
