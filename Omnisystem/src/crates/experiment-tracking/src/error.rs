//! Error types for experiment tracking

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ExperimentError {
    #[error("experiment not found")]
    ExperimentNotFound,

    #[error("run not found")]
    RunNotFound,

    #[error("other error: {0}")]
    Other(String),
}

pub type ExperimentResult<T> = std::result::Result<T, ExperimentError>;
