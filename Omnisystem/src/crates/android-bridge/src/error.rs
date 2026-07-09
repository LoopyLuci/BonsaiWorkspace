//! Error types

#[derive(Debug, Clone)]
pub enum Error {
    /// Device pool / connection state is invalid for the requested operation
    InvalidState(String),
    /// Capability token issuance, verification, or lookup failed
    CapabilityError(String),
    /// JSON (de)serialization failed
    SerializationError(String),
    /// Encryption/decryption failed
    CryptoError(String),
    /// Channel send/receive failed
    CommunicationError(String),
    /// Device discovery/registration failed
    DiscoveryError(String),
    /// File path handling failed
    PathError(String),
    /// Input event injection failed
    InputError(String),
    /// Screen streaming failed
    StreamingError(String),
    /// Filesystem I/O failed
    Io(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidState(msg) => write!(f, "invalid state: {}", msg),
            Error::CapabilityError(msg) => write!(f, "capability error: {}", msg),
            Error::SerializationError(msg) => write!(f, "serialization error: {}", msg),
            Error::CryptoError(msg) => write!(f, "crypto error: {}", msg),
            Error::CommunicationError(msg) => write!(f, "communication error: {}", msg),
            Error::DiscoveryError(msg) => write!(f, "discovery error: {}", msg),
            Error::PathError(msg) => write!(f, "path error: {}", msg),
            Error::InputError(msg) => write!(f, "input error: {}", msg),
            Error::StreamingError(msg) => write!(f, "streaming error: {}", msg),
            Error::Io(msg) => write!(f, "I/O error: {}", msg),
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<std::time::SystemTimeError> for Error {
    fn from(e: std::time::SystemTimeError) -> Self {
        Error::Io(e.to_string())
    }
}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
