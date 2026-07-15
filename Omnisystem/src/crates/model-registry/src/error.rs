//! Error types for the model registry.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// The requested model could not be found.
    ModelNotFound,
    /// Creating or fetching a model version failed.
    VersioningFailed,
    /// Promoting a version to a new stage failed.
    PromotionFailed,
    /// A training job operation failed.
    TrainingFailed,
    /// A deployment operation failed.
    DeploymentFailed,
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::ModelNotFound => write!(f, "model not found"),
            RegistryError::VersioningFailed => write!(f, "model version not found or versioning failed"),
            RegistryError::PromotionFailed => write!(f, "version promotion failed"),
            RegistryError::TrainingFailed => write!(f, "training job not found or operation failed"),
            RegistryError::DeploymentFailed => write!(f, "deployment not found or operation failed"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Result type used throughout the model registry.
pub type RegistryResult<T> = std::result::Result<T, RegistryError>;
