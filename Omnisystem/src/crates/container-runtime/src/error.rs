//! Container-runtime specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("container already running: {0}")]
    ContainerAlreadyRunning(String),
    #[error("container not found: {0}")]
    ContainerNotFound(String),
    #[error("container not running: {0}")]
    ContainerNotRunning(String),
    #[error("image already exists: {0}")]
    ImageAlreadyExists(String),
    #[error("image not found: {0}")]
    ImageNotFound(String),
    #[error("registry error: {0}")]
    RegistryError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
