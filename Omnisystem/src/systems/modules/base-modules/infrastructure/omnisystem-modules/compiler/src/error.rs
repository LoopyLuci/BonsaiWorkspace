//! Error types for Omnisystem Compiler Module

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Compilation error: {0}")]
    CompilationError(String),

    #[error("Language not supported: {0}")]
    LanguageNotSupported(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Distributed compilation error: {0}")]
    DistributedError(String),

    #[error("IDE integration error: {0}")]
    IDEError(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("Build error: {0}")]
    BuildError(String),
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Error::CompilationError(err.to_string())
    }
}
