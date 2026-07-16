//! Error types

#[derive(Debug, Clone)]
pub enum AggregationError {
    /// No samples have been recorded for the requested metric name
    MetricNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for AggregationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregationError::MetricNotFound => write!(f, "no samples recorded for this metric"),
            AggregationError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for AggregationError {}

/// Result type
pub type AggregationResult<T> = std::result::Result<T, AggregationError>;
