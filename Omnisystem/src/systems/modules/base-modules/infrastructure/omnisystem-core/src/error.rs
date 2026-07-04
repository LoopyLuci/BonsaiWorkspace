//! Error types for Omnisystem Core

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Module error: {0}")]
    ModuleError(String),

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Module already registered: {0}")]
    ModuleAlreadyExists(String),

    #[error("Capability not found: {0}")]
    CapabilityNotFound(String),

    #[error("Dependency error: {0}")]
    DependencyError(String),

    #[error("Capability disabled: {0}")]
    CapabilityDisabled(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Data error: {0}")]
    DataError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("Module initialization failed: {module_name} - {reason}")]
    InitializationFailed {
        module_name: String,
        reason: String,
    },

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl Error {
    pub fn module_error(msg: impl Into<String>) -> Self {
        Error::ModuleError(msg.into())
    }

    pub fn capability_error(name: impl Into<String>) -> Self {
        Error::CapabilityNotFound(name.into())
    }

    pub fn initialization_failed(module: &str, reason: &str) -> Self {
        Error::InitializationFailed {
            module_name: module.to_string(),
            reason: reason.to_string(),
        }
    }
}
