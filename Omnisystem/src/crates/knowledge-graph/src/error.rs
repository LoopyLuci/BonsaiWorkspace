//! Error types

#[derive(Debug, Clone)]
pub enum GraphError {
    /// Referenced entity does not exist
    EntityNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::EntityNotFound => write!(f, "entity not found"),
            GraphError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for GraphError {}

/// Result type
pub type GraphResult<T> = std::result::Result<T, GraphError>;
