//! Comprehensive error types for SRWSTS operations
//!
//! Defines all possible error scenarios that can occur during test execution,
//! fault injection, and result collection, with proper context and recovery hints.

use thiserror::Error;
use uuid::Uuid;

/// Result type for SRWSTS operations
pub type SrwstsResult<T> = std::result::Result<T, SrwstsError>;

/// Comprehensive error type for SRWSTS operations
#[derive(Debug, Error)]
pub enum SrwstsError {
    // ============= Initialization Errors =============
    #[error("Failed to initialize SRWSTS environment: {reason}")]
    InitializationFailed { reason: String },

    #[error("Invalid configuration: {reason}")]
    InvalidConfiguration { reason: String },

    #[error("Invalid test plan: {reason}")]
    InvalidTestPlan { reason: String },

    // ============= Test Execution Errors =============
    #[error("Test execution failed: {reason}")]
    ExecutionFailed { reason: String },

    #[error("Test timed out after {duration_secs}s")]
    TestTimeout { duration_secs: u64 },

    #[error("Test execution was cancelled")]
    ExecutionCancelled,

    #[error("Test not found: {test_id}")]
    TestNotFound { test_id: String },

    #[error("Test execution already in progress for {test_id}")]
    TestAlreadyRunning { test_id: String },

    // ============= Fault Injection Errors =============
    #[error("Failed to inject fault: {reason}")]
    FaultInjectionFailed { reason: String },

    #[error("Unsupported fault type: {fault_type}")]
    UnsupportedFaultType { fault_type: String },

    #[error("Invalid fault parameters: {reason}")]
    InvalidFaultParameters { reason: String },

    #[error("Fault injection not enabled")]
    FaultInjectionDisabled,

    #[error("Fault recovery failed: {reason}")]
    FaultRecoveryFailed { reason: String },

    // ============= Resource Errors =============
    #[error("Insufficient resources: {reason}")]
    InsufficientResources { reason: String },

    #[error("Resource limit exceeded: {resource_type} limit {limit} exceeded with {actual}")]
    ResourceLimitExceeded {
        resource_type: String,
        limit: u64,
        actual: u64,
    },

    #[error("Resource allocation failed: {reason}")]
    ResourceAllocationFailed { reason: String },

    #[error("Resource cleanup failed: {reason}")]
    ResourceCleanupFailed { reason: String },

    // ============= Result Collection Errors =============
    #[error("Failed to collect test result: {reason}")]
    ResultCollectionFailed { reason: String },

    #[error("Result storage error: {reason}")]
    ResultStorageError { reason: String },

    #[error("Result not found: {result_id}")]
    ResultNotFound { result_id: Uuid },

    #[error("Result validation failed: {reason}")]
    ResultValidationFailed { reason: String },

    // ============= Metrics Errors =============
    #[error("Failed to collect metrics: {reason}")]
    MetricsCollectionFailed { reason: String },

    #[error("Metrics validation failed: {reason}")]
    MetricsValidationFailed { reason: String },

    #[error("Invalid metric value: {metric_name} = {value}")]
    InvalidMetricValue {
        metric_name: String,
        value: String,
    },

    // ============= Concurrency Errors =============
    #[error("Concurrency limit exceeded: {current} >= {limit}")]
    ConcurrencyLimitExceeded { current: usize, limit: usize },

    #[error("Failed to acquire execution lock: {reason}")]
    LockAcquisitionFailed { reason: String },

    #[error("Channel send failed: {reason}")]
    ChannelSendFailed { reason: String },

    // ============= IO Errors =============
    #[error("IO error: {reason}")]
    IoError { reason: String },

    #[error("Failed to read file: {path}: {reason}")]
    FileReadError { path: String, reason: String },

    #[error("Failed to write file: {path}: {reason}")]
    FileWriteError { path: String, reason: String },

    #[error("Directory creation failed: {path}: {reason}")]
    DirectoryCreationFailed { path: String, reason: String },

    // ============= State Errors =============
    #[error("Invalid state transition: from {from_state} to {to_state}")]
    InvalidStateTransition {
        from_state: String,
        to_state: String,
    },

    #[error("State machine error: {reason}")]
    StateMachineError { reason: String },

    // ============= Assertion/Verification Errors =============
    #[error("Assertion failed: {reason}")]
    AssertionFailed { reason: String },

    #[error("Expected value {expected} but got {actual}")]
    ValueMismatch { expected: String, actual: String },

    #[error("Output comparison failed: {reason}")]
    OutputComparisonFailed { reason: String },

    // ============= External Integration Errors =============
    #[error("Failed to communicate with external service: {service}: {reason}")]
    ExternalServiceError {
        service: String,
        reason: String,
    },

    #[error("Kernel API call failed: {reason}")]
    KernelApiError { reason: String },

    #[error("Network error: {reason}")]
    NetworkError { reason: String },

    // ============= Generic/Unknown Errors =============
    #[error("Unknown error: {reason}")]
    Unknown { reason: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl SrwstsError {
    /// Create a new initialization error
    pub fn init_failed(reason: impl Into<String>) -> Self {
        Self::InitializationFailed {
            reason: reason.into(),
        }
    }

    /// Create a new execution failed error
    pub fn execution_failed(reason: impl Into<String>) -> Self {
        Self::ExecutionFailed {
            reason: reason.into(),
        }
    }

    /// Create a new fault injection error
    pub fn fault_injection_failed(reason: impl Into<String>) -> Self {
        Self::FaultInjectionFailed {
            reason: reason.into(),
        }
    }

    /// Create a new resource error
    pub fn insufficient_resources(reason: impl Into<String>) -> Self {
        Self::InsufficientResources {
            reason: reason.into(),
        }
    }

    /// Create a new result collection error
    pub fn result_collection_failed(reason: impl Into<String>) -> Self {
        Self::ResultCollectionFailed {
            reason: reason.into(),
        }
    }

    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::TestTimeout { .. })
    }

    /// Check if this is a resource limit error
    pub fn is_resource_limit(&self) -> bool {
        matches!(
            self,
            Self::ResourceLimitExceeded { .. } | Self::InsufficientResources { .. }
        )
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::TestTimeout { .. }
                | Self::ExecutionCancelled
                | Self::FaultRecoveryFailed { .. }
                | Self::ConcurrencyLimitExceeded { .. }
                | Self::ResourceAllocationFailed { .. }
        )
    }

    /// Get a recovery suggestion for this error
    pub fn recovery_suggestion(&self) -> &'static str {
        match self {
            Self::TestTimeout { .. } => "Increase timeout duration or optimize test performance",
            Self::ExecutionCancelled => "Retry the test execution",
            Self::ConcurrencyLimitExceeded { .. } => {
                "Increase max_concurrent or wait for other tests to complete"
            }
            Self::ResourceAllocationFailed { .. } => {
                "Free up system resources and retry"
            }
            Self::InsufficientResources { .. } => "Allocate more resources to the test environment",
            Self::FaultRecoveryFailed { .. } => "Check system state and manually recover if needed",
            _ => "Check logs for more details",
        }
    }
}

/// Convert from std::io::Error
impl From<std::io::Error> for SrwstsError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            reason: err.to_string(),
        }
    }
}

/// Convert from serde_json::Error
impl From<serde_json::Error> for SrwstsError {
    fn from(err: serde_json::Error) -> Self {
        Self::Unknown {
            reason: format!("JSON serialization error: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_error_detection() {
        let err = SrwstsError::TestTimeout {
            duration_secs: 300,
        };
        assert!(err.is_timeout());
    }

    #[test]
    fn test_resource_limit_error_detection() {
        let err = SrwstsError::ResourceLimitExceeded {
            resource_type: "memory".to_string(),
            limit: 1000,
            actual: 2000,
        };
        assert!(err.is_resource_limit());
    }

    #[test]
    fn test_recoverable_error_detection() {
        let timeout = SrwstsError::TestTimeout {
            duration_secs: 300,
        };
        assert!(timeout.is_recoverable());

        let invalid_config = SrwstsError::InvalidConfiguration {
            reason: "missing field".to_string(),
        };
        assert!(!invalid_config.is_recoverable());
    }

    #[test]
    fn test_recovery_suggestions() {
        let err = SrwstsError::ConcurrencyLimitExceeded {
            current: 100,
            limit: 100,
        };
        let suggestion = err.recovery_suggestion();
        assert!(suggestion.contains("max_concurrent"));
    }

    #[test]
    fn test_error_helpers() {
        let err = SrwstsError::init_failed("test setup failed");
        assert!(matches!(err, SrwstsError::InitializationFailed { .. }));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let srwsts_err = SrwstsError::from(io_err);
        assert!(matches!(srwsts_err, SrwstsError::IoError { .. }));
    }
}
