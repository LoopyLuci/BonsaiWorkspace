//! Error types

#[derive(Debug, Clone)]
pub enum EventSourcingError {
    /// Other error
    Other(String),
}

impl std::fmt::Display for EventSourcingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSourcingError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for EventSourcingError {}

/// Result type
pub type EventSourcingResult<T> = std::result::Result<T, EventSourcingError>;
