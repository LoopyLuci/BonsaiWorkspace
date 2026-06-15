//! Bootstrap system for loading Bonsai Ecosystem on Omnisystem

mod config;
mod loader;
mod state;

pub use config::BootstrapConfig;
pub use loader::ApplicationBootstrap;
pub use state::{EcosystemState, ComponentState, ComponentInfo};

use crate::errors::ApplicationStressError;
use async_trait::async_trait;

/// Result type for bootstrap operations
pub type BootstrapResult<T> = Result<T, BootstrapError>;

/// Bootstrap-specific error types
#[derive(Debug, Clone)]
pub enum BootstrapError {
    ImageLoadFailed(String),
    ComponentInitializationFailed(String),
    DependencyResolutionFailed(String),
    HealthCheckFailed(String),
    TimeoutError(String),
}

impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapError::ImageLoadFailed(msg) => write!(f, "Image load failed: {}", msg),
            BootstrapError::ComponentInitializationFailed(msg) => {
                write!(f, "Component initialization failed: {}", msg)
            }
            BootstrapError::DependencyResolutionFailed(msg) => {
                write!(f, "Dependency resolution failed: {}", msg)
            }
            BootstrapError::HealthCheckFailed(msg) => write!(f, "Health check failed: {}", msg),
            BootstrapError::TimeoutError(msg) => write!(f, "Timeout: {}", msg),
        }
    }
}

impl std::error::Error for BootstrapError {}

impl From<BootstrapError> for ApplicationStressError {
    fn from(err: BootstrapError) -> Self {
        ApplicationStressError::Bootstrap(err.to_string())
    }
}

/// Trait for bootstrapping ecosystem components
#[async_trait]
pub trait BootstrapComponent: Send + Sync {
    /// Get component name
    fn name(&self) -> &str;

    /// Initialize component
    async fn initialize(&self) -> BootstrapResult<()>;

    /// Verify component health
    async fn health_check(&self) -> BootstrapResult<()>;

    /// Cleanup component
    async fn cleanup(&self) -> BootstrapResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_error_display() {
        let err = BootstrapError::ImageLoadFailed("image not found".to_string());
        assert_eq!(err.to_string(), "Image load failed: image not found");
    }

    #[test]
    fn test_bootstrap_error_to_application_error() {
        let err = BootstrapError::ComponentInitializationFailed("init failed".to_string());
        let app_err: ApplicationStressError = err.into();
        assert_eq!(
            app_err.to_string(),
            "Bootstrap error: Component initialization failed: init failed"
        );
    }

    #[test]
    fn test_health_check_error_display() {
        let err = BootstrapError::HealthCheckFailed("service not responding".to_string());
        assert_eq!(err.to_string(), "Health check failed: service not responding");
    }
}
