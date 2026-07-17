//! Error types

#[derive(Debug, Clone)]
pub enum Error {
    /// Other error
    Other(String),
    /// Record not found
    NotFound(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Other(msg) => write!(f, "Error: {}", msg),
            Error::NotFound(id) => write!(f, "Not found: {}", id),
        }
    }
}

impl std::error::Error for Error {}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
