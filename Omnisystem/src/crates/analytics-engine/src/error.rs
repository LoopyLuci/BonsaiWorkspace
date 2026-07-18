//! Error types

#[derive(Debug, Clone)]
pub enum AnalyticsError {
    /// No data points found for the requested dataset
    DatasetNotFound,
    /// Requested aggregation type is not supported
    AggregationFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyticsError::DatasetNotFound => write!(f, "dataset not found"),
            AnalyticsError::AggregationFailed => write!(f, "aggregation failed"),
            AnalyticsError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for AnalyticsError {}

/// Result type
pub type AnalyticsResult<T> = std::result::Result<T, AnalyticsError>;
