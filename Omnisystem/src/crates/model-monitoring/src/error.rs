//! Error types

#[derive(Debug, Clone)]
pub enum MonitoringError {
    /// A drift/anomaly score was NaN or outside the valid [0.0, 1.0] range
    InvalidScore,
    /// Other error
    Other(String),
}

impl std::fmt::Display for MonitoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitoringError::InvalidScore => write!(f, "score must be a finite value in [0.0, 1.0]"),
            MonitoringError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for MonitoringError {}

/// Result type
pub type MonitoringResult<T> = std::result::Result<T, MonitoringError>;
