//! Error types

#[derive(Debug, Clone)]
pub enum InfraError {
    /// Other error
    Other(String),
}

impl std::fmt::Display for InfraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InfraError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for InfraError {}

/// Result type
pub type InfraResult<T> = std::result::Result<T, InfraError>;
