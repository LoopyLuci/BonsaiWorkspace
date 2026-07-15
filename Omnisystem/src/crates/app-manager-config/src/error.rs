//! Error types for app configuration management.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration not found for app: {0}")]
    ConfigNotFound(String),
    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result type used across the crate.
pub type Result<T> = std::result::Result<T, ConfigError>;
