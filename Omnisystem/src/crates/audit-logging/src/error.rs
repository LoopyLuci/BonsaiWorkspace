//! Error types for audit logging

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum AuditError {
    #[error("audit log not found")]
    LogNotFound,

    #[error("other error: {0}")]
    Other(String),
}

pub type AuditResult<T> = std::result::Result<T, AuditError>;
