//! Tenant-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum TenantError {
    #[error("access denied")]
    AccessDenied,
    #[error("tenant not found")]
    TenantNotFound,
}

pub type TenantResult<T> = std::result::Result<T, TenantError>;
