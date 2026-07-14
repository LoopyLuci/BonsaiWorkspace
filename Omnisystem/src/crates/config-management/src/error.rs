//! Config-management specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ConfigError {
    #[error("feature flag not found")]
    FeatureFlagNotFound,
    #[error("config not found")]
    ConfigNotFound,
}

pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
