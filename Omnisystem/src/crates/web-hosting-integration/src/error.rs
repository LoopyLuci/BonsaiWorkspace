//! Web-hosting integration specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum IntegrationError {
    #[error("failover error: {0}")]
    FailoverError(String),
    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("configuration error: {0}")]
    ConfigurationError(String),
    #[error("security violation: {0}")]
    SecurityViolation(String),
    #[error("rate limit exceeded")]
    RateLimitExceeded,
    #[error("internal error: {0}")]
    Internal(String),
}

pub type IntegrationResult<T> = std::result::Result<T, IntegrationError>;
