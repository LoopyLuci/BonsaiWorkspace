//! Error types for this component
use std::fmt;
use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Component error type
#[derive(Error, Debug)]
pub enum Error {
    /// Generic error with message
    #[error("Component error: {0}")]
    Other(String),
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
