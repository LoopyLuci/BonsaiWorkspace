//! Error types for application stress testing

use thiserror::Error;

/// Result type for application stress testing operations
pub type ApplicationStressResult<T> = Result<T, ApplicationStressError>;

/// Comprehensive error types for application stress testing
#[derive(Debug, Error)]
pub enum ApplicationStressError {
    #[error("Bootstrap error: {0}")]
    Bootstrap(String),

    #[error("Test execution error: {0}")]
    TestExecution(String),

    #[error("Workspace error: {0}")]
    WorkspaceError(String),

    #[error("Buddy error: {0}")]
    BuddyError(String),

    #[error("Omni-Bot error: {0}")]
    OmniBotError(String),

    #[error("CRDT merge error: {0}")]
    CrdtMergeError(String),

    #[error("File operation error: {0}")]
    FileOperationError(String),

    #[error("Compilation error: {0}")]
    CompilationError(String),

    #[error("Synchronization error: {0}")]
    SyncError(String),

    #[error("Memory error: {0}")]
    MemoryError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Metric collection error: {0}")]
    MetricsError(String),

    #[error("Input simulation error: {0}")]
    InputSimulationError(String),

    #[error("Fault injection error: {0}")]
    FaultInjectionError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Data corruption: {0}")]
    DataCorruption(String),

    #[error("State inconsistency: {0}")]
    StateInconsistency(String),

    #[error("Verification failure: {0}")]
    VerificationFailure(String),

    #[error("Async operation error: {0}")]
    AsyncError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<anyhow::Error> for ApplicationStressError {
    fn from(err: anyhow::Error) -> Self {
        ApplicationStressError::Unknown(err.to_string())
    }
}

impl From<tokio::task::JoinError> for ApplicationStressError {
    fn from(err: tokio::task::JoinError) -> Self {
        ApplicationStressError::AsyncError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_error() {
        let err = ApplicationStressError::Bootstrap("failed to load ecosystem".to_string());
        assert_eq!(err.to_string(), "Bootstrap error: failed to load ecosystem");
    }

    #[test]
    fn test_test_execution_error() {
        let err = ApplicationStressError::TestExecution("test failed".to_string());
        assert_eq!(err.to_string(), "Test execution error: test failed");
    }

    #[test]
    fn test_crdt_merge_error() {
        let err = ApplicationStressError::CrdtMergeError("conflict resolution failed".to_string());
        assert_eq!(err.to_string(), "CRDT merge error: conflict resolution failed");
    }

    #[test]
    fn test_data_corruption_error() {
        let err = ApplicationStressError::DataCorruption("checksum mismatch".to_string());
        assert_eq!(err.to_string(), "Data corruption: checksum mismatch");
    }

    #[test]
    fn test_timeout_error() {
        let err = ApplicationStressError::Timeout("operation exceeded 10s".to_string());
        assert_eq!(err.to_string(), "Timeout: operation exceeded 10s");
    }
}
