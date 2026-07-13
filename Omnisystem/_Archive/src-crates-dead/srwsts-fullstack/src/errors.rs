//! Error types for full-stack testing

use thiserror::Error;

/// Result type for full-stack testing operations
pub type FullStackTestResult<T> = Result<T, FullStackTestError>;

/// Comprehensive error type for full-stack testing
#[derive(Error, Debug)]
pub enum FullStackTestError {
    #[error("Bootstrap failed: {0}")]
    BootstrapFailed(String),

    #[error("Vault initialization failed: {0}")]
    VaultInitFailed(String),

    #[error("Kernel panic: {0}")]
    KernelPanic(String),

    #[error("Service crash: {service}, error: {error}")]
    ServiceCrash { service: String, error: String },

    #[error("Application failure: {app}, error: {error}")]
    AppFailure { app: String, error: String },

    #[error("Test timeout after {secs} seconds")]
    TestTimeout { secs: u64 },

    #[error("Resource exhaustion: {0}")]
    ResourceExhaustion(String),

    #[error("Memory allocation failed: {0}")]
    MemoryAllocationFailed(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Network partition detected: {0}")]
    NetworkPartition(String),

    #[error("State inconsistency: expected {expected}, got {actual}")]
    StateInconsistency { expected: String, actual: String },

    #[error("Data corruption detected: {0}")]
    DataCorruption(String),

    #[error("Audit log mismatch: {0}")]
    AuditLogMismatch(String),

    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    #[error("Deterministic replay diverged: {0}")]
    ReplayDivergence(String),

    #[error("Performance degradation exceeded threshold: {metric} = {actual}, expected <= {threshold}")]
    PerformanceDegradation {
        metric: String,
        actual: f64,
        threshold: f64,
    },

    #[error("Starvation detected: low-priority tasks blocked for {secs} seconds")]
    TaskStarvation { secs: u64 },

    #[error("Deadlock detected: {0}")]
    Deadlock(String),

    #[error("Fault injection failed: {0}")]
    FaultInjectionFailed(String),

    #[error("CRDT drift detected: {nodes} nodes, divergence = {divergence}")]
    CrdtDrift { nodes: usize, divergence: f64 },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Test execution error: {0}")]
    ExecutionError(String),

    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl FullStackTestError {
    /// Create a bootstrap error
    pub fn bootstrap(msg: impl Into<String>) -> Self {
        Self::BootstrapFailed(msg.into())
    }

    /// Create a vault initialization error
    pub fn vault_init(msg: impl Into<String>) -> Self {
        Self::VaultInitFailed(msg.into())
    }

    /// Create a kernel panic error
    pub fn kernel_panic(msg: impl Into<String>) -> Self {
        Self::KernelPanic(msg.into())
    }

    /// Create a service crash error
    pub fn service_crash(service: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ServiceCrash {
            service: service.into(),
            error: error.into(),
        }
    }

    /// Create an application failure error
    pub fn app_failure(app: impl Into<String>, error: impl Into<String>) -> Self {
        Self::AppFailure {
            app: app.into(),
            error: error.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(secs: u64) -> Self {
        Self::TestTimeout { secs }
    }

    /// Create a state inconsistency error
    pub fn state_inconsistency(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::StateInconsistency {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a network partition error
    pub fn network_partition(msg: impl Into<String>) -> Self {
        Self::NetworkPartition(msg.into())
    }

    /// Is this a fatal error?
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::KernelPanic(_)
                | Self::Deadlock(_)
                | Self::DataCorruption(_)
                | Self::ResourceExhaustion(_)
        )
    }

    /// Is this a transient error?
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::NetworkPartition(_) | Self::TestTimeout { .. } | Self::ServiceCrash { .. }
        )
    }

    /// Is this a consistency error?
    pub fn is_consistency_error(&self) -> bool {
        matches!(
            self,
            Self::StateInconsistency { .. }
                | Self::DataCorruption(_)
                | Self::AuditLogMismatch(_)
                | Self::CrdtDrift { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = FullStackTestError::bootstrap("test");
        assert!(matches!(err, FullStackTestError::BootstrapFailed(_)));
    }

    #[test]
    fn test_error_severity() {
        let fatal = FullStackTestError::kernel_panic("test");
        assert!(fatal.is_fatal());

        let transient = FullStackTestError::timeout(10);
        assert!(transient.is_transient());

        let consistency = FullStackTestError::state_inconsistency("A", "B");
        assert!(consistency.is_consistency_error());
    }

    #[test]
    fn test_error_display() {
        let err = FullStackTestError::timeout(30);
        let display = format!("{}", err);
        assert!(display.contains("30"));
    }
}
