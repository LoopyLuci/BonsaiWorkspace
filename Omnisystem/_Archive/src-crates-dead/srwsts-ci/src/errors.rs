//! Error types for SRWSTS-CI

use thiserror::Error;

pub type CIResult<T> = Result<T, CIError>;

/// Comprehensive error type for CI operations
#[derive(Error, Debug)]
pub enum CIError {
    #[error("Baseline not found: {0}")]
    BaselineNotFound(String),

    #[error("Baseline integrity check failed: expected {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    #[error("Baseline approval required: {0}")]
    ApprovalRequired(String),

    #[error("Regression detected: {details}")]
    RegressionDetected { details: String },

    #[error("Performance threshold exceeded: {metric} {value} vs baseline {baseline} (threshold: {threshold}%)")]
    PerformanceThresholdExceeded {
        metric: String,
        value: f64,
        baseline: f64,
        threshold: f64,
    },

    #[error("Correctness regression: test {test_name} failed (baseline: passed)")]
    CorrectnessRegression { test_name: String },

    #[error("Determinism regression: test {test_name} variance detected")]
    DeterminismRegression { test_name: String },

    #[error("Test execution failed: {0}")]
    TestExecutionFailed(String),

    #[error("Metrics collection failed: {0}")]
    MetricsCollectionFailed(String),

    #[error("Report generation failed: {0}")]
    ReportGenerationFailed(String),

    #[error("Artifact collection failed: {0}")]
    ArtifactCollectionFailed(String),

    #[error("Alert delivery failed: {target} - {reason}")]
    AlertDeliveryFailed { target: String, reason: String },

    #[error("CAS operation failed: {0}")]
    CasError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid baseline version: {0}")]
    InvalidBaselineVersion(String),

    #[error("Approval workflow error: {0}")]
    ApprovalWorkflowError(String),

    #[error("Invalid metric: {0}")]
    InvalidMetric(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl CIError {
    pub fn is_regression(&self) -> bool {
        matches!(
            self,
            CIError::RegressionDetected { .. }
                | CIError::PerformanceThresholdExceeded { .. }
                | CIError::CorrectnessRegression { .. }
                | CIError::DeterminismRegression { .. }
        )
    }

    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            CIError::TestExecutionFailed(_)
                | CIError::AlertDeliveryFailed { .. }
                | CIError::MetricsCollectionFailed(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regression_errors() {
        let err = CIError::RegressionDetected {
            details: "test failure".to_string(),
        };
        assert!(err.is_regression());
    }

    #[test]
    fn test_recoverable_errors() {
        let err = CIError::TestExecutionFailed("timeout".to_string());
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_performance_threshold_error() {
        let err = CIError::PerformanceThresholdExceeded {
            metric: "latency_p99".to_string(),
            value: 150.0,
            baseline: 100.0,
            threshold: 5.0,
        };
        assert!(err.is_regression());
        assert!(err.to_string().contains("latency_p99"));
    }

    #[test]
    fn test_integrity_check_error() {
        let err = CIError::IntegrityCheckFailed {
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert!(!err.is_regression());
        assert!(err.to_string().contains("abc123"));
    }
}
