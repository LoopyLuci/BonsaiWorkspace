//! Error types for the data pipeline crate.

#[derive(Debug, Clone)]
pub enum PipelineError {
    /// A stage failed while extracting data from a source.
    ExtractError(String),
    /// A stage failed while transforming data.
    TransformError(String),
    /// A stage failed while loading data into a destination.
    LoadError(String),
    /// Catch-all for anything else.
    Other(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::ExtractError(msg) => write!(f, "extract error: {}", msg),
            PipelineError::TransformError(msg) => write!(f, "transform error: {}", msg),
            PipelineError::LoadError(msg) => write!(f, "load error: {}", msg),
            PipelineError::Other(msg) => write!(f, "pipeline error: {}", msg),
        }
    }
}

impl std::error::Error for PipelineError {}

/// Result type used throughout the data pipeline crate.
pub type Result<T> = std::result::Result<T, PipelineError>;

// Backwards-compatible alias (matches the original stub's naming).
pub type Error = PipelineError;
