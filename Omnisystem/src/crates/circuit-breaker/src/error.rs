//! Circuit-breaker specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum CircuitBreakerError {
    #[error("circuit is open")]
    CircuitOpen,
    #[error("circuit not found")]
    CircuitNotFound,
    #[error("circuit recovery failed")]
    RecoveryFailed,
    #[error("invalid configuration")]
    InvalidConfiguration,
    #[error("health check failed")]
    HealthCheckFailed,
}

pub type CircuitBreakerResult<T> = std::result::Result<T, CircuitBreakerError>;
