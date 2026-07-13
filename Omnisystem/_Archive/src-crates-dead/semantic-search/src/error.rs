//! Error types

#[derive(Debug, Clone)]
pub enum SemanticError {
    /// Vectors being compared have mismatched dimensions
    SimilarityFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemanticError::SimilarityFailed => write!(f, "vector dimensions do not match"),
            SemanticError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for SemanticError {}

/// Result type
pub type SemanticResult<T> = std::result::Result<T, SemanticError>;
