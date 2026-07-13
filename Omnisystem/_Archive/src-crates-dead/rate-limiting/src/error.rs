//! Error types

#[derive(Debug, Clone)]
pub enum RateLimitError {
    /// Bucket not found
    BucketNotFound,
    /// Quota exceeded
    QuotaExceeded,
    /// Other error
    Other(String),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::BucketNotFound => write!(f, "bucket not found"),
            RateLimitError::QuotaExceeded => write!(f, "quota exceeded"),
            RateLimitError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for RateLimitError {}

/// Result type
pub type RateLimitResult<T> = std::result::Result<T, RateLimitError>;
