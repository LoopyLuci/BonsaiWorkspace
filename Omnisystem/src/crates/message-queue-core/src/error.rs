//! Error types

#[derive(Debug, Clone)]
pub enum QueueError {
    /// Topic not found
    TopicNotFound,
    /// Partition not found
    PartitionNotFound,
    /// Requested offset is out of range
    OffsetOutOfRange,
    /// Other error
    Other(String),
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::TopicNotFound => write!(f, "topic not found"),
            QueueError::PartitionNotFound => write!(f, "partition not found"),
            QueueError::OffsetOutOfRange => write!(f, "offset out of range"),
            QueueError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for QueueError {}

/// Result type
pub type QueueResult<T> = std::result::Result<T, QueueError>;
