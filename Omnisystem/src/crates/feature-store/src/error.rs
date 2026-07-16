//! Error types

#[derive(Debug, Clone)]
pub enum FeatureError {
    /// Requested feature or feature group does not exist
    FeatureNotFound,
    /// No stored value matches the requested feature/entity pair
    RetrievalFailed,
    /// No active version exists for the requested feature
    VersioningFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for FeatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureError::FeatureNotFound => write!(f, "feature or feature group not found"),
            FeatureError::RetrievalFailed => write!(f, "no value found for this feature/entity pair"),
            FeatureError::VersioningFailed => write!(f, "no active version found for this feature"),
            FeatureError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for FeatureError {}

/// Result type
pub type FeatureResult<T> = std::result::Result<T, FeatureError>;
