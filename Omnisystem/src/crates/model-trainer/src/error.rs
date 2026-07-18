//! Error types for the model-trainer crate.

#[derive(Debug, Clone)]
pub enum TrainerError {
    /// A requested model doesn't exist, or a model-related operation failed.
    ModelError(String),
    /// Validation input was inconsistent (e.g. mismatched slice lengths).
    ValidationError(String),
    /// Catch-all for anything else.
    Other(String),
}

impl std::fmt::Display for TrainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrainerError::ModelError(msg) => write!(f, "model error: {}", msg),
            TrainerError::ValidationError(msg) => write!(f, "validation error: {}", msg),
            TrainerError::Other(msg) => write!(f, "model-trainer error: {}", msg),
        }
    }
}

impl std::error::Error for TrainerError {}

/// Result type used throughout the model-trainer crate.
pub type Result<T> = std::result::Result<T, TrainerError>;

// Backwards-compatible alias (matches the original stub's naming).
pub type Error = TrainerError;
