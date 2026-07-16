//! Error types

#[derive(Debug, Clone)]
pub enum AnalyticsError {
    /// Referenced pipeline does not exist
    PipelineExecutionFailed,
    /// A data record was submitted with no fields
    InvalidData,
    /// Aggregation was requested against an unknown pipeline
    AggregationFailed,
    /// Schema validation failed (unknown schema or empty record)
    SchemaValidationFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyticsError::PipelineExecutionFailed => write!(f, "pipeline not found or failed to execute"),
            AnalyticsError::InvalidData => write!(f, "data record has no fields"),
            AnalyticsError::AggregationFailed => write!(f, "cannot aggregate: unknown pipeline"),
            AnalyticsError::SchemaValidationFailed => write!(f, "schema validation failed"),
            AnalyticsError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for AnalyticsError {}

/// Result type
pub type AnalyticsResult<T> = std::result::Result<T, AnalyticsError>;
