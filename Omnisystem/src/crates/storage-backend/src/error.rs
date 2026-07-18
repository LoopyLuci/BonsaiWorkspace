//! Error types

#[derive(Debug, Clone)]
pub enum StorageError {
    /// Key not found in the memtable
    KeyNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::KeyNotFound => write!(f, "key not found"),
            StorageError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

/// Result type
pub type StorageResult<T> = std::result::Result<T, StorageError>;
