//! Error types

#[derive(Debug, Clone)]
pub enum FeatureError {
    /// Feature not found
    FeatureNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for FeatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureError::FeatureNotFound => write!(f, "feature not found"),
            FeatureError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for FeatureError {}

/// Result type
pub type FeatureResult<T> = std::result::Result<T, FeatureError>;
