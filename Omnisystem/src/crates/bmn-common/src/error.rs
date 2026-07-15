//! Error types for BMN (Broadcast Media Network)

/// Errors produced by BMN sources, encoders, and transports
#[derive(Debug, Clone)]
pub enum BmnError {
    /// A source-related failure (device unavailable, already started,
    /// platform capture not implemented, ...)
    SourceError(String),
    /// Attempted to send on a transport that isn't connected
    TransportNotConnected,
    /// Encoder failure
    EncoderError(String),
    /// Internal invariant violation (e.g. render called before init)
    Internal(String),
    /// Other error
    Other(String),
}

impl BmnError {
    /// Construct a [`BmnError::SourceError`]
    pub fn source_error(msg: impl Into<String>) -> Self {
        BmnError::SourceError(msg.into())
    }

    /// Construct a [`BmnError::Internal`]
    pub fn internal(msg: impl Into<String>) -> Self {
        BmnError::Internal(msg.into())
    }
}

impl std::fmt::Display for BmnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BmnError::SourceError(msg) => write!(f, "source error: {}", msg),
            BmnError::TransportNotConnected => write!(f, "transport not connected"),
            BmnError::EncoderError(msg) => write!(f, "encoder error: {}", msg),
            BmnError::Internal(msg) => write!(f, "internal error: {}", msg),
            BmnError::Other(msg) => write!(f, "error: {}", msg),
        }
    }
}

impl std::error::Error for BmnError {}

/// Result type
pub type BmnResult<T> = std::result::Result<T, BmnError>;
