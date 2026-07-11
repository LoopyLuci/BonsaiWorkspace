//! Error types

#[derive(Debug, Clone)]
pub enum OrchestrationError {
    /// Pod not found
    PodNotFound(String),
    /// Deployment not found
    DeploymentNotFound(String),
    /// Health check failed
    HealthCheckFailed(String),
    /// Configuration error
    ConfigurationError(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for OrchestrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestrationError::PodNotFound(id) => write!(f, "pod not found: {}", id),
            OrchestrationError::DeploymentNotFound(id) => write!(f, "deployment not found: {}", id),
            OrchestrationError::HealthCheckFailed(msg) => write!(f, "health check failed: {}", msg),
            OrchestrationError::ConfigurationError(msg) => write!(f, "configuration error: {}", msg),
            OrchestrationError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for OrchestrationError {}

/// Result type
pub type OrchestrationResult<T> = std::result::Result<T, OrchestrationError>;
