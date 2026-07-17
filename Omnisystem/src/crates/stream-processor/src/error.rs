//! Error types for stream processing

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum StreamError {
    #[error("windowing failed: window not found")]
    WindowingFailed,

    #[error("aggregation failed: unsupported aggregation type")]
    AggregationFailed,

    #[error("state management failed: key not found")]
    StateManagementFailed,

    #[error("other error: {0}")]
    Other(String),
}

pub type StreamResult<T> = std::result::Result<T, StreamError>;
