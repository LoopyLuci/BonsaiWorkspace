//! Web-hosting specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum WebError {
    #[error("configuration error: {0}")]
    ConfigurationError(String),
    #[error("virtual host not found: {0}")]
    VirtualHostNotFound(String),
    #[error("virtual host already exists: {0}")]
    VirtualHostAlreadyExists(String),
    #[error("certificate not found: {0}")]
    CertificateNotFound(String),
}

pub type WebResult<T> = std::result::Result<T, WebError>;
