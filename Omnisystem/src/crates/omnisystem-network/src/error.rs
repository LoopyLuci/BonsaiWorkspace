//! Error types for the network utilities.

#[derive(Debug)]
pub enum NetworkError {
    /// JSON encode/decode failure in the protocol handler.
    Serialization(serde_json::Error),
    /// Any other network error.
    Other(String),
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::Serialization(e) => write!(f, "serialization error: {}", e),
            NetworkError::Other(msg) => write!(f, "network error: {}", msg),
        }
    }
}

impl std::error::Error for NetworkError {}

impl From<serde_json::Error> for NetworkError {
    fn from(e: serde_json::Error) -> Self {
        NetworkError::Serialization(e)
    }
}

/// Result type used throughout the network utilities.
pub type Result<T> = std::result::Result<T, NetworkError>;
