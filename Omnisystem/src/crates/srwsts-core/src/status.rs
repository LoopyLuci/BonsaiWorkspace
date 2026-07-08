//! Status and result enumeration types for test execution
//!
//! Provides enums for tracking test execution state and overall result status.

use serde::{Deserialize, Serialize};

/// Execution status for a test
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Test is queued but not yet started
    Pending,
    /// Test is currently running
    Running,
    /// Test completed successfully
    Passed,
    /// Test completed but assertions failed
    Failed,
    /// Test was cancelled before completion
    Cancelled,
    /// Test encountered an error during execution
    Error,
    /// Test timed out
    Timeout,
    /// Test was skipped
    Skipped,
}

impl ExecutionStatus {
    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Passed | Self::Failed | Self::Cancelled | Self::Error | Self::Timeout | Self::Skipped
        )
    }

    /// Check if this status represents success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Passed)
    }

    /// Check if this status represents failure
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Error | Self::Timeout | Self::Cancelled
        )
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Pending => "Waiting to start execution",
            Self::Running => "Currently executing",
            Self::Passed => "Test passed",
            Self::Failed => "Test assertions failed",
            Self::Cancelled => "Test was cancelled",
            Self::Error => "Test encountered an error",
            Self::Timeout => "Test execution timed out",
            Self::Skipped => "Test was skipped",
        }
    }
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Running => write!(f, "RUNNING"),
            Self::Passed => write!(f, "PASSED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Cancelled => write!(f, "CANCELLED"),
            Self::Error => write!(f, "ERROR"),
            Self::Timeout => write!(f, "TIMEOUT"),
            Self::Skipped => write!(f, "SKIPPED"),
        }
    }
}

impl Default for ExecutionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Overall result status for a test execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResultStatus {
    /// All checks and assertions passed
    Pass,
    /// One or more assertions failed
    Fail,
    /// Execution encountered an error
    Error,
    /// Test timed out
    Timeout,
    /// Test was cancelled
    Cancelled,
    /// Test could not be executed
    NotRun,
}

impl ResultStatus {
    /// Check if this represents success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Check if this represents failure
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Fail | Self::Error | Self::Timeout | Self::Cancelled | Self::NotRun
        )
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Pass => "All assertions and checks passed",
            Self::Fail => "One or more assertions failed",
            Self::Error => "An error occurred during test execution",
            Self::Timeout => "Test execution timed out",
            Self::Cancelled => "Test execution was cancelled",
            Self::NotRun => "Test was not executed",
        }
    }
}

impl std::fmt::Display for ResultStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Fail => write!(f, "FAIL"),
            Self::Error => write!(f, "ERROR"),
            Self::Timeout => write!(f, "TIMEOUT"),
            Self::Cancelled => write!(f, "CANCELLED"),
            Self::NotRun => write!(f, "NOT_RUN"),
        }
    }
}

impl Default for ResultStatus {
    fn default() -> Self {
        Self::NotRun
    }
}

/// Outcome of a fault injection attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaultOutcome {
    /// Fault was successfully injected
    Injected,
    /// System recovered from the fault
    Recovered,
    /// System did not recover from the fault
    NotRecovered,
    /// Fault injection was skipped
    Skipped,
}

impl FaultOutcome {
    /// Check if system recovered
    pub fn recovered(&self) -> bool {
        matches!(self, Self::Recovered)
    }
}

impl std::fmt::Display for FaultOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Injected => write!(f, "INJECTED"),
            Self::Recovered => write!(f, "RECOVERED"),
            Self::NotRecovered => write!(f, "NOT_RECOVERED"),
            Self::Skipped => write!(f, "SKIPPED"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_status_is_terminal() {
        assert!(!ExecutionStatus::Pending.is_terminal());
        assert!(!ExecutionStatus::Running.is_terminal());
        assert!(ExecutionStatus::Passed.is_terminal());
        assert!(ExecutionStatus::Failed.is_terminal());
    }

    #[test]
    fn test_execution_status_is_success() {
        assert!(ExecutionStatus::Passed.is_success());
        assert!(!ExecutionStatus::Failed.is_success());
    }

    #[test]
    fn test_execution_status_is_failure() {
        assert!(ExecutionStatus::Failed.is_failure());
        assert!(ExecutionStatus::Timeout.is_failure());
        assert!(!ExecutionStatus::Passed.is_failure());
    }

    #[test]
    fn test_result_status_is_success() {
        assert!(ResultStatus::Pass.is_success());
        assert!(!ResultStatus::Fail.is_success());
    }

    #[test]
    fn test_result_status_is_failure() {
        assert!(ResultStatus::Fail.is_failure());
        assert!(ResultStatus::Error.is_failure());
        assert!(!ResultStatus::Pass.is_failure());
    }

    #[test]
    fn test_status_display() {
        assert_eq!(ExecutionStatus::Pending.to_string(), "PENDING");
        assert_eq!(ResultStatus::Pass.to_string(), "PASS");
    }

    #[test]
    fn test_fault_outcome_recovered() {
        assert!(FaultOutcome::Recovered.recovered());
        assert!(!FaultOutcome::NotRecovered.recovered());
    }
}
