//! Error types for the replication manager

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ReplicationError {
    #[error("replica not found")]
    ReplicaNotFound,

    #[error("failover policy not found")]
    PolicyNotFound,

    #[error("other error: {0}")]
    Other(String),
}

pub type ReplicationResult<T> = std::result::Result<T, ReplicationError>;
