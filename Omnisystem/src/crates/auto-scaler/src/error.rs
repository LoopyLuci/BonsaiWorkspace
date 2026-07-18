//! Error types for auto-scaler

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalingError {
    /// No scaling policy has been registered for the requested service.
    PolicyNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for ScalingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalingError::PolicyNotFound => write!(f, "no scaling policy registered for this service"),
            ScalingError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ScalingError {}

/// Result type for scaling operations
pub type ScalingResult<T> = std::result::Result<T, ScalingError>;
