//! Error types for the connector-core crate.

#[derive(Debug, Clone)]
pub enum ConnectorError {
    /// A connector with the given id/name already exists in a registry.
    AlreadyExists(String),
    /// No connector/pending request found for the given id.
    NotFound(String),
    /// An arena (or other bounded buffer) allocation failed.
    AllocationFailed(String),
    /// A request/reply operation timed out waiting for a response.
    Timeout,
    /// A channel (mpsc/oneshot/broadcast) was closed unexpectedly.
    ChannelClosed,
    /// Serialization/deserialization failure.
    Serialization(String),
    /// Catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectorError::AlreadyExists(id) => write!(f, "connector already exists: {id}"),
            ConnectorError::NotFound(id) => write!(f, "connector not found: {id}"),
            ConnectorError::AllocationFailed(msg) => write!(f, "allocation failed: {msg}"),
            ConnectorError::Timeout => write!(f, "operation timed out"),
            ConnectorError::ChannelClosed => write!(f, "channel closed"),
            ConnectorError::Serialization(msg) => write!(f, "serialization error: {msg}"),
            ConnectorError::Other(msg) => write!(f, "error: {msg}"),
        }
    }
}

impl std::error::Error for ConnectorError {}

impl From<serde_json::Error> for ConnectorError {
    fn from(err: serde_json::Error) -> Self {
        ConnectorError::Serialization(err.to_string())
    }
}

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, ConnectorError>;
