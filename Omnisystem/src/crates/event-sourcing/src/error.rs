//! Error types

#[derive(Debug, Clone)]
pub enum EventSourcingError {
    /// `start_version` was greater than `end_version` in a replay request
    InvalidVersionRange { start: u32, end: u32 },
    /// Other error
    Other(String),
}

impl std::fmt::Display for EventSourcingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventSourcingError::InvalidVersionRange { start, end } => {
                write!(f, "invalid version range: start {} > end {}", start, end)
            }
            EventSourcingError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for EventSourcingError {}

/// Result type
pub type EventSourcingResult<T> = std::result::Result<T, EventSourcingError>;
