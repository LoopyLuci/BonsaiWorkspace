//! Audit-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum AuditError {
    #[error("audit query failed")]
    QueryFailed,
}

pub type AuditResult<T> = std::result::Result<T, AuditError>;
