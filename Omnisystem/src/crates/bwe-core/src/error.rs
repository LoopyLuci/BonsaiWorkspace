//! Error types for the Bonsai Web Engine (BWE) core.

#[derive(Debug, thiserror::Error)]
pub enum BweError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Custom(String),
}

/// Result type used across the crate.
pub type Result<T> = std::result::Result<T, BweError>;
