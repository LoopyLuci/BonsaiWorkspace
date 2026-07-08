//! Test result types and storage
//!
//! Provides comprehensive types for capturing and storing test execution results,
//! including assertions, system state, and detailed error information.

use crate::{types::Timestamp, RunId, TestId, TestMetrics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Single assertion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    /// Unique assertion ID
    pub id: String,
    /// Description of what is being asserted
    pub description: String,
    /// Whether the assertion passed
    pub passed: bool,
    /// Expected value (for comparison assertions)
    pub expected: Option<String>,
    /// Actual value (for comparison assertions)
    pub actual: Option<String>,
    /// Error message if assertion failed
    pub error_message: Option<String>,
}

impl AssertionResult {
    /// Create a new assertion result
    pub fn new(id: impl Into<String>, description: impl Into<String>, passed: bool) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            passed,
            expected: None,
            actual: None,
            error_message: None,
        }
    }

    /// Add expected and actual values to this assertion
    pub fn with_values(
        mut self,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        self.expected = Some(expected.into());
        self.actual = Some(actual.into());
        self
    }

    /// Add an error message to this assertion
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error_message = Some(error.into());
        self
    }
}

/// Record of a fault injection event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultEvent {
    /// Unique fault ID
    pub fault_id: String,
    /// Timestamp when fault was injected
    pub injected_at: Timestamp,
    /// Whether recovery was detected
    pub recovered: bool,
    /// Timestamp when recovery was detected (if applicable)
    pub recovered_at: Option<Timestamp>,
    /// System state when fault was detected
    pub system_state: HashMap<String, String>,
    /// Any recovery actions taken
    pub recovery_actions: Vec<String>,
}

impl FaultEvent {
    /// Create a new fault event
    pub fn new(fault_id: impl Into<String>, injected_at: Timestamp) -> Self {
        Self {
            fault_id: fault_id.into(),
            injected_at,
            recovered: false,
            recovered_at: None,
            system_state: HashMap::new(),
            recovery_actions: Vec::new(),
        }
    }

    /// Mark this fault as recovered
    pub fn mark_recovered(mut self, recovered_at: Timestamp) -> Self {
        self.recovered = true;
        self.recovered_at = Some(recovered_at);
        self
    }

    /// Add system state information
    pub fn with_state(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.system_state.insert(key.into(), value.into());
        self
    }

    /// Add a recovery action
    pub fn add_recovery_action(mut self, action: impl Into<String>) -> Self {
        self.recovery_actions.push(action.into());
        self
    }

    /// Get recovery time in seconds (if recovered)
    pub fn recovery_time_secs(&self) -> Option<f64> {
        if let Some(recovered_at) = self.recovered_at {
            recovered_at.elapsed().map(|d| d.as_secs_f64())
        } else {
            None
        }
    }
}

/// Complete test execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Unique result ID
    pub result_id: Uuid,
    /// ID of the test that produced this result
    pub test_id: TestId,
    /// ID of the test run
    pub run_id: RunId,
    /// Timestamp when test execution started
    pub started_at: Timestamp,
    /// Timestamp when test execution completed
    pub completed_at: Timestamp,
    /// Overall test status
    pub status: crate::status::ResultStatus,
    /// Assertions from the test
    pub assertions: Vec<AssertionResult>,
    /// Fault events that occurred
    pub fault_events: Vec<FaultEvent>,
    /// Metrics collected during execution
    pub metrics: TestMetrics,
    /// Captured output from the test
    pub stdout: String,
    /// Captured error output
    pub stderr: String,
    /// Any error that occurred during execution
    pub execution_error: Option<String>,
    /// Custom result data
    pub custom_data: HashMap<String, serde_json::Value>,
}

impl TestResult {
    /// Create a new test result
    pub fn new(test_id: TestId, run_id: RunId, started_at: Timestamp) -> Self {
        Self {
            result_id: Uuid::new_v4(),
            test_id,
            run_id,
            started_at,
            completed_at: Timestamp::now(),
            status: crate::status::ResultStatus::NotRun,
            assertions: Vec::new(),
            fault_events: Vec::new(),
            metrics: TestMetrics::new(),
            stdout: String::new(),
            stderr: String::new(),
            execution_error: None,
            custom_data: HashMap::new(),
        }
    }

    /// Mark this result as passed
    pub fn mark_passed(mut self) -> Self {
        self.status = crate::status::ResultStatus::Pass;
        self
    }

    /// Mark this result as failed with optional message
    pub fn mark_failed(mut self, message: Option<impl Into<String>>) -> Self {
        self.status = crate::status::ResultStatus::Fail;
        if let Some(msg) = message {
            self.execution_error = Some(msg.into());
        }
        self
    }

    /// Mark this result as error
    pub fn mark_error(mut self, error: impl Into<String>) -> Self {
        self.status = crate::status::ResultStatus::Error;
        self.execution_error = Some(error.into());
        self
    }

    /// Add an assertion result
    pub fn add_assertion(mut self, assertion: AssertionResult) -> Self {
        self.assertions.push(assertion);
        self
    }

    /// Add a fault event
    pub fn add_fault_event(mut self, event: FaultEvent) -> Self {
        self.fault_events.push(event);
        self
    }

    /// Set standard output
    pub fn set_stdout(mut self, output: impl Into<String>) -> Self {
        self.stdout = output.into();
        self
    }

    /// Set standard error output
    pub fn set_stderr(mut self, output: impl Into<String>) -> Self {
        self.stderr = output.into();
        self
    }

    /// Add custom data
    pub fn with_custom_data(
        mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        self.custom_data.insert(key.into(), value);
        self
    }

    /// Calculate execution duration
    pub fn duration_secs(&self) -> f64 {
        (self.completed_at.to_system_time()
            .duration_since(self.started_at.to_system_time())
            .unwrap_or_default())
        .as_secs_f64()
    }

    /// Get number of passed assertions
    pub fn assertions_passed(&self) -> usize {
        self.assertions.iter().filter(|a| a.passed).count()
    }

    /// Get number of failed assertions
    pub fn assertions_failed(&self) -> usize {
        self.assertions.iter().filter(|a| !a.passed).count()
    }

    /// Get assertion pass rate as percentage
    pub fn assertion_pass_rate(&self) -> f64 {
        if self.assertions.is_empty() {
            return 100.0;
        }
        (self.assertions_passed() as f64 / self.assertions.len() as f64) * 100.0
    }

    /// Check if result is successful
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Validate the result
    pub fn validate(&self) -> crate::errors::SrwstsResult<()> {
        use crate::errors::SrwstsError;

        if self.completed_at.to_system_time() < self.started_at.to_system_time() {
            return Err(SrwstsError::ResultValidationFailed {
                reason: "completed_at must be >= started_at".to_string(),
            });
        }

        self.metrics.validate()?;

        Ok(())
    }

    /// Convert to JSON
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Convert to compact JSON
    pub fn to_json_compact(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_result_creation() {
        let result = AssertionResult::new("a1", "should be 42", true);
        assert!(result.passed);
        assert_eq!(result.id, "a1");
    }

    #[test]
    fn test_assertion_result_with_values() {
        let result = AssertionResult::new("a1", "comparison", false)
            .with_values("expected", "actual")
            .with_error("values don't match");
        assert_eq!(result.expected, Some("expected".to_string()));
        assert_eq!(result.actual, Some("actual".to_string()));
        assert!(result.error_message.is_some());
    }

    #[test]
    fn test_fault_event_creation() {
        let event = FaultEvent::new("f1", Timestamp::now());
        assert!(!event.recovered);
        assert_eq!(event.fault_id, "f1");
    }

    #[test]
    fn test_fault_event_mark_recovered() {
        let event = FaultEvent::new("f1", Timestamp::now())
            .mark_recovered(Timestamp::now());
        assert!(event.recovered);
        assert!(event.recovered_at.is_some());
    }

    #[test]
    fn test_test_result_creation() {
        let test_id = TestId::new("test1");
        let run_id = RunId::new();
        let result = TestResult::new(test_id, run_id, Timestamp::now());
        assert_eq!(result.status, crate::status::ResultStatus::NotRun);
    }

    #[test]
    fn test_test_result_mark_passed() {
        let test_id = TestId::new("test1");
        let run_id = RunId::new();
        let result = TestResult::new(test_id, run_id, Timestamp::now())
            .mark_passed();
        assert!(result.is_success());
    }

    #[test]
    fn test_test_result_assertions() {
        let test_id = TestId::new("test1");
        let run_id = RunId::new();
        let result = TestResult::new(test_id, run_id, Timestamp::now())
            .add_assertion(AssertionResult::new("a1", "test 1", true))
            .add_assertion(AssertionResult::new("a2", "test 2", false));
        assert_eq!(result.assertions_passed(), 1);
        assert_eq!(result.assertions_failed(), 1);
        assert!(result.assertion_pass_rate() > 49.0 && result.assertion_pass_rate() < 51.0);
    }

    #[test]
    fn test_test_result_duration() {
        let test_id = TestId::new("test1");
        let run_id = RunId::new();
        let start = Timestamp::now();
        let result = TestResult::new(test_id, run_id, start);
        let duration = result.duration_secs();
        assert!(duration >= 0.0);
    }

    #[test]
    fn test_test_result_json_serialization() {
        let test_id = TestId::new("test1");
        let run_id = RunId::new();
        let result = TestResult::new(test_id, run_id, Timestamp::now())
            .mark_passed();
        let json = result.to_json();
        assert!(json.is_ok());
    }
}
