//! Error types for network policy management

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum PolicyError {
    #[error("network policy not found")]
    PolicyNotFound,

    #[error("mTLS configuration failed: policy not found")]
    MtlsConfigurationFailed,

    #[error("access denied: no matching access control rule")]
    AccessDenied,

    #[error("certificate not found or invalid")]
    CertificateInvalid,

    #[error("network segment not found")]
    SegmentNotFound,

    #[error("other error: {0}")]
    Other(String),
}

pub type PolicyResult<T> = std::result::Result<T, PolicyError>;
