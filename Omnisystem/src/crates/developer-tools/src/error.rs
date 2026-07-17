//! Developer-tools specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DevToolError {
    #[error("generation failed")]
    GenerationFailed,
}

pub type DevToolResult<T> = std::result::Result<T, DevToolError>;
