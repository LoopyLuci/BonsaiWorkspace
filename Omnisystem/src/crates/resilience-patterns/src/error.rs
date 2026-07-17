//! Resilience-pattern specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ResilienceError {
    #[error("bulkhead limit exceeded: {0} active calls")]
    BulkheadLimitExceeded(usize),
    #[error("retries exhausted")]
    RetriesExhausted,
    #[error("internal error: {0}")]
    Internal(String),
}

pub type ResilienceResult<T> = std::result::Result<T, ResilienceError>;
