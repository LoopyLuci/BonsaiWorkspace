//! Error types

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// The requested key is absent (never set, evicted, or TTL-expired)
    KeyNotFound,
    /// A replication operation failed (e.g. removing an unknown node)
    ReplicationFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::KeyNotFound => write!(f, "key not found"),
            CacheError::ReplicationFailed => write!(f, "replication operation failed"),
            CacheError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for CacheError {}

/// Result type
pub type CacheResult<T> = std::result::Result<T, CacheError>;
