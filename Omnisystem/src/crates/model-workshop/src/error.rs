//! Error types for the Model Workshop backend.

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("module not found: {0}")]
    ModuleNotFound(String),
    #[error("dataset not found: {0}")]
    DatasetNotFound(String),
    #[error("training job not found: {0}")]
    JobNotFound(String),
    #[error("invalid model config: {0}")]
    InvalidConfig(String),
    #[error("{0}")]
    Other(String),
}

/// Result type used across the crate.
pub type Result<T> = std::result::Result<T, Error>;
