use thiserror::Error;

#[derive(Debug, Error)]
pub enum DfError {
    #[error("polars error: {0}")]
    Polars(#[from] polars::prelude::PolarsError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("column not found: {0}")]
    ColumnNotFound(String),
    #[error("type mismatch: {0}")]
    TypeMismatch(String),
    #[error("{0}")]
    Other(String),
}

pub type DfResult<T> = Result<T, DfError>;
