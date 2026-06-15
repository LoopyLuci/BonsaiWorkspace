//! Error types for language frontends

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FrontendError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Effect error: {0}")]
    EffectError(String),

    #[error("Lowering error: {0}")]
    LoweringError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("LAIR error: {0}")]
    LairError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Unknown language: {0}")]
    UnknownLanguage(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type Result<T> = std::result::Result<T, FrontendError>;
