//! Lifecycle-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum LifecycleError {
    #[error("cluster already registered: {0}")]
    ClusterAlreadyRegistered(String),
    #[error("cluster not found: {0}")]
    ClusterNotFound(String),
    #[error("no previous revision to roll back to")]
    NoPreviousRevision,
    #[error("rollback failed: {0}")]
    RollbackFailed(String),
    #[error("rollout not found: {0}")]
    RolloutNotFound(String),
}

pub type LifecycleResult<T> = std::result::Result<T, LifecycleError>;
