//! Error types for the submodule system

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum SubModuleError {
    #[error("module '{0}' is already loaded")]
    AlreadyLoaded(String),

    #[error("module '{0}' not found")]
    NotFound(String),

    #[error("version mismatch: required {required}, found {actual}")]
    VersionMismatch { required: String, actual: String },

    #[error("other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SubModuleError>;
