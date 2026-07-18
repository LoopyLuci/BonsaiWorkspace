//! Error types for complex event processing

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum CEPError {
    #[error("pattern not found")]
    PatternNotFound,

    #[error("pattern matching failed: match not found")]
    MatchingFailed,

    #[error("other error: {0}")]
    Other(String),
}

pub type CEPResult<T> = std::result::Result<T, CEPError>;
