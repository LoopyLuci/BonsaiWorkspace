//! Error types for the performance monitor.

#[derive(Debug, Clone, PartialEq)]
pub enum MonitorError {
    /// A metrics calculation was given invalid input (empty series, an
    /// out-of-range percentile, ...).
    MetricsError(String),
    /// Any other monitor error.
    Other(String),
}

impl std::fmt::Display for MonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitorError::MetricsError(msg) => write!(f, "metrics error: {}", msg),
            MonitorError::Other(msg) => write!(f, "monitor error: {}", msg),
        }
    }
}

impl std::error::Error for MonitorError {}

/// Result type used throughout the performance monitor.
pub type Result<T> = std::result::Result<T, MonitorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages() {
        assert_eq!(
            MonitorError::MetricsError("empty".into()).to_string(),
            "metrics error: empty"
        );
    }
}
