//! Error types for service stress testing

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type for service operations
pub type ServiceResult<T> = Result<T, ServiceError>;

/// Errors that can occur during service stress testing
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[serde(tag = "error_type")]
pub enum ServiceError {
    /// Service initialization failed
    #[error("Service initialization failed: {reason}")]
    InitializationFailed { reason: String },

    /// Service not found
    #[error("Service not found: {service_name}")]
    ServiceNotFound { service_name: String },

    /// Service startup timeout
    #[error("Service startup timeout: {service_name} (timeout: {timeout_secs}s)")]
    StartupTimeout { service_name: String, timeout_secs: u64 },

    /// Service crashed or became unavailable
    #[error("Service crashed: {service_name} - {reason}")]
    ServiceCrashed { service_name: String, reason: String },

    /// Bootstrap failed
    #[error("Bootstrap failed: {reason}")]
    BootstrapFailed { reason: String },

    /// Configuration error
    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    /// Test execution failed
    #[error("Test execution failed: {test_name} - {reason}")]
    TestExecutionFailed { test_name: String, reason: String },

    /// Fault injection failed
    #[error("Fault injection failed: {fault_type} - {reason}")]
    FaultInjectionFailed { fault_type: String, reason: String },

    /// Fault recovery failed
    #[error("Fault recovery failed: {reason}")]
    FaultRecoveryFailed { reason: String },

    /// Metrics collection error
    #[error("Metrics collection error: {reason}")]
    MetricsError { reason: String },

    /// P2P mesh convergence timeout
    #[error("P2P mesh convergence timeout: {nodes_remaining} nodes unreachable after {timeout_secs}s")]
    MeshConvergenceTimeout { nodes_remaining: usize, timeout_secs: u64 },

    /// Multi-path bonding failure
    #[error("Multi-path bonding failure: {reason}")]
    BondingFailure { reason: String },

    /// Storage deduplication failure
    #[error("Storage deduplication failure: {reason}")]
    DeduplicationFailure { reason: String },

    /// Erasure code reconstruction failure
    #[error("Erasure code reconstruction failure: {reason}")]
    ReconstructionFailure { reason: String },

    /// Network stack error
    #[error("Network stack error: {reason}")]
    NetworkStackError { reason: String },

    /// Firewall rule error
    #[error("Firewall rule matching error: {reason}")]
    FirewallError { reason: String },

    /// GPU memory exhaustion
    #[error("GPU memory exhaustion: {required_mb}MB required, {available_mb}MB available")]
    GpuMemoryExhaustion { required_mb: u64, available_mb: u64 },

    /// GPU reset recovery failed
    #[error("GPU reset recovery failed: {reason}")]
    GpuResetFailed { reason: String },

    /// Service discovery error
    #[error("Service discovery error: {reason}")]
    DiscoveryError { reason: String },

    /// DNS resolution error
    #[error("DNS resolution error: {reason}")]
    DnsResolutionError { reason: String },

    /// Health check failure
    #[error("Health check failure: {service_name} - {reason}")]
    HealthCheckFailed { service_name: String, reason: String },

    /// Service interaction error
    #[error("Service interaction error: {reason}")]
    InteractionError { reason: String },

    /// Cascading failure detected
    #[error("Cascading failure: {service_name} dependent on {dependent_service}")]
    CascadingFailure { service_name: String, dependent_service: String },

    /// Timeout during operation
    #[error("Operation timeout: {reason}")]
    Timeout { reason: String },

    /// Backpressure handling error
    #[error("Backpressure error: {reason}")]
    BackpressureError { reason: String },

    /// Service snapshot error
    #[error("Service snapshot error: {reason}")]
    SnapshotError { reason: String },

    /// Service restore error
    #[error("Service restore error: {reason}")]
    RestoreError { reason: String },

    /// State consistency error
    #[error("State consistency error: {reason}")]
    StateConsistencyError { reason: String },

    /// IO error
    #[error("IO error: {reason}")]
    IoError { reason: String },

    /// Internal error
    #[error("Internal error: {reason}")]
    Internal { reason: String },
}

impl ServiceError {
    /// Create a new initialization failed error
    pub fn init_failed(reason: impl Into<String>) -> Self {
        Self::InitializationFailed {
            reason: reason.into(),
        }
    }

    /// Create a new service not found error
    pub fn not_found(service_name: impl Into<String>) -> Self {
        Self::ServiceNotFound {
            service_name: service_name.into(),
        }
    }

    /// Create a new test execution failed error
    pub fn test_failed(test_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::TestExecutionFailed {
            test_name: test_name.into(),
            reason: reason.into(),
        }
    }

    /// Create a new fault injection failed error
    pub fn injection_failed(fault_type: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::FaultInjectionFailed {
            fault_type: fault_type.into(),
            reason: reason.into(),
        }
    }

    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(
            self,
            Self::StartupTimeout { .. }
                | Self::MeshConvergenceTimeout { .. }
                | Self::Timeout { .. }
        )
    }

    /// Check if this is a cascading failure
    pub fn is_cascading(&self) -> bool {
        matches!(self, Self::CascadingFailure { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = ServiceError::init_failed("test reason");
        assert!(matches!(error, ServiceError::InitializationFailed { .. }));
    }

    #[test]
    fn test_error_serialization() {
        let error = ServiceError::not_found("test-service");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("test-service"));
    }

    #[test]
    fn test_is_timeout() {
        let error = ServiceError::StartupTimeout {
            service_name: "test".to_string(),
            timeout_secs: 30,
        };
        assert!(error.is_timeout());
    }

    #[test]
    fn test_is_cascading() {
        let error = ServiceError::CascadingFailure {
            service_name: "a".to_string(),
            dependent_service: "b".to_string(),
        };
        assert!(error.is_cascading());
    }
}
