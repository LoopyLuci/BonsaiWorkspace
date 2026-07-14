//! Coordination-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum CoordinationError {
    #[error("conflict detected")]
    ConflictDetected,
    #[error("lock acquisition failed")]
    LockAcquisitionFailed,
    #[error("invalid step")]
    InvalidStep,
    #[error("invalid phase")]
    InvalidPhase,
    #[error("saga not found")]
    SagaNotFound,
    #[error("transaction not found")]
    TransactionNotFound,
}

pub type CoordinationResult<T> = std::result::Result<T, CoordinationError>;
