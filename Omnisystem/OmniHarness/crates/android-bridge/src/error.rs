//! Error types

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("{0}")]
    Other(String),
    #[error("discovery error: {0}")]
    DiscoveryError(String),
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("capability error: {0}")]
    CapabilityError(String),
    #[error("input error: {0}")]
    InputError(String),
    #[error("communication error: {0}")]
    CommunicationError(String),
    #[error("streaming error: {0}")]
    StreamingError(String),
    #[error("crypto error: {0}")]
    CryptoError(String),
    #[error("path error: {0}")]
    PathError(String),
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("system time error: {0}")]
    SystemTime(#[from] std::time::SystemTimeError),
}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
