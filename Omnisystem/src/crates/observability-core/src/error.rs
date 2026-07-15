//! Error types

/// Errors produced by the tracing, logging, metrics, and correlation backends
#[derive(Debug, Clone)]
pub enum ObservabilityError {
    /// No span exists with the given id
    SpanNotFound(String),
    /// No trace exists with the given id
    TraceNotFound(String),
    /// No correlation context exists for the given id
    CorrelationIdMissing,
    /// A metric value was invalid (e.g. NaN/infinite)
    InvalidMetric(String),
    /// Metrics aggregation could not be completed
    MetricsAggregationFailed(String),
    /// A log write/batch failed
    LogCollectionFailed(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for ObservabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservabilityError::SpanNotFound(id) => write!(f, "span not found: {}", id),
            ObservabilityError::TraceNotFound(id) => write!(f, "trace not found: {}", id),
            ObservabilityError::CorrelationIdMissing => write!(f, "correlation id missing"),
            ObservabilityError::InvalidMetric(msg) => write!(f, "invalid metric: {}", msg),
            ObservabilityError::MetricsAggregationFailed(msg) => {
                write!(f, "metrics aggregation failed: {}", msg)
            }
            ObservabilityError::LogCollectionFailed(msg) => {
                write!(f, "log collection failed: {}", msg)
            }
            ObservabilityError::Other(msg) => write!(f, "error: {}", msg),
        }
    }
}

impl std::error::Error for ObservabilityError {}

/// Result type
pub type ObservabilityResult<T> = std::result::Result<T, ObservabilityError>;
