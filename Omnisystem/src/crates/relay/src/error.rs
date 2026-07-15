//! Error types for the blind relay server/client.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("relay session is full")]
    SessionFull,
    #[error("proof-of-work verification failed")]
    PowFailed,
    #[error("invalid relay token")]
    InvalidToken,
    #[error("frame too large: {0} bytes")]
    FrameTooLarge(usize),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

/// Result type used across the crate.
pub type RelayResult<T> = std::result::Result<T, RelayError>;
