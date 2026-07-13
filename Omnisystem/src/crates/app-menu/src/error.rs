//! Error types

#[derive(Debug, Clone)]
pub enum AppMenuError {
    /// Other error
    Other(String),
}

impl std::fmt::Display for AppMenuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppMenuError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for AppMenuError {}

/// Result type
pub type AppMenuResult<T> = std::result::Result<T, AppMenuError>;
