//! Error types

#[derive(Debug, Clone)]
pub enum AlertingError {
    /// No notification route is registered for the requested severity
    RoutingFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for AlertingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertingError::RoutingFailed => write!(f, "no notification route registered for this severity"),
            AlertingError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for AlertingError {}

/// Result type
pub type AlertingResult<T> = std::result::Result<T, AlertingError>;
