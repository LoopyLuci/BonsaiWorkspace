//! Error types

#[derive(Debug, Clone)]
pub enum StorageError {
    /// Object not found
    ObjectNotFound(String),
    /// Bucket not found
    BucketNotFound(String),
    /// Bucket already exists
    BucketAlreadyExists(String),
    /// Invalid offset
    InvalidOffset(String),
    /// Permission denied
    PermissionDenied(String),
    /// Block not found
    BlockNotFound(String),
    /// Insufficient space
    InsufficientSpace(String),
    /// Invalid block size
    InvalidBlockSize(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::ObjectNotFound(msg) => write!(f, "object not found: {}", msg),
            StorageError::BucketNotFound(msg) => write!(f, "bucket not found: {}", msg),
            StorageError::BucketAlreadyExists(msg) => write!(f, "bucket already exists: {}", msg),
            StorageError::InvalidOffset(msg) => write!(f, "invalid offset: {}", msg),
            StorageError::PermissionDenied(msg) => write!(f, "permission denied: {}", msg),
            StorageError::BlockNotFound(msg) => write!(f, "block not found: {}", msg),
            StorageError::InsufficientSpace(msg) => write!(f, "insufficient space: {}", msg),
            StorageError::InvalidBlockSize(msg) => write!(f, "invalid block size: {}", msg),
            StorageError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

/// Result type
pub type StorageResult<T> = std::result::Result<T, StorageError>;
