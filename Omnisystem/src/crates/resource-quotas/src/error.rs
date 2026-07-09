//! Error types

#[derive(Debug, Clone)]
pub enum QuotaError {
    /// Quota not found for the requested tenant
    QuotaNotFound,
    /// Tenant not found
    TenantNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for QuotaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuotaError::QuotaNotFound => write!(f, "quota not found"),
            QuotaError::TenantNotFound => write!(f, "tenant not found"),
            QuotaError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for QuotaError {}

/// Result type
pub type QuotaResult<T> = std::result::Result<T, QuotaError>;
