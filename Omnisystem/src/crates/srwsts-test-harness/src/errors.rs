//! Error types for the test harness

use thiserror::Error;

/// Result type for harness operations
pub type HarnessResult<T> = Result<T, HarnessError>;

/// Test harness errors
#[derive(Debug, Clone, Error)]
pub enum HarnessError {
    /// Vault creation failed
    #[error("Failed to create vault: {0}")]
    VaultCreationFailed(String),

    /// Vault not found
    #[error("Vault not found: {0}")]
    VaultNotFound(String),

    /// Test execution failed
    #[error("Test execution failed: {0}")]
    ExecutionFailed(String),

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    /// Timeout during test execution
    #[error("Test execution timeout")]
    Timeout,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Snapshot failed
    #[error("Snapshot failed: {0}")]
    SnapshotFailed(String),

    /// Restore failed
    #[error("Restore failed: {0}")]
    RestoreFailed(String),

    /// Metric collection failed
    #[error("Metric collection failed: {0}")]
    MetricsError(String),

    /// Trace recording failed
    #[error("Trace recording failed: {0}")]
    TraceError(String),

    /// Test harness not running
    #[error("Test harness not running")]
    NotRunning,

    /// Generic harness error
    #[error("Harness error: {0}")]
    Other(String),
}

impl From<std::io::Error> for HarnessError {
    fn from(err: std::io::Error) -> Self {
        HarnessError::IoError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_creation_failed_error() {
        let err = HarnessError::VaultCreationFailed("out of memory".to_string());
        assert!(err.to_string().contains("out of memory"));
    }

    #[test]
    fn test_resource_limit_exceeded_error() {
        let err = HarnessError::ResourceLimitExceeded("memory".to_string());
        assert!(err.to_string().contains("memory"));
    }

    #[test]
    fn test_timeout_error() {
        let err: HarnessError = HarnessError::Timeout;
        assert!(err.to_string().contains("timeout"));
    }
}
