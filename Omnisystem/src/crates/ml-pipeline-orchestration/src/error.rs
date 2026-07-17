//! Error types for the ML pipeline orchestrator

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum PipelineError {
    #[error("pipeline not found")]
    PipelineNotFound,

    #[error("execution failed: execution not found")]
    ExecutionFailed,

    #[error("cyclic task dependency detected in pipeline")]
    CyclicDependency,

    #[error("other error: {0}")]
    Other(String),
}

pub type PipelineResult<T> = std::result::Result<T, PipelineError>;
