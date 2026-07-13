//! Error types

#[derive(Debug, Clone)]
pub enum ObservabilityError {
    /// Trace not found
    TraceNotFound,
    /// Span not found
    SpanNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for ObservabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservabilityError::TraceNotFound => write!(f, "trace not found"),
            ObservabilityError::SpanNotFound => write!(f, "span not found"),
            ObservabilityError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ObservabilityError {}

/// Result type
pub type ObservabilityResult<T> = std::result::Result<T, ObservabilityError>;
