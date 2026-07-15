//! Error types

/// Errors produced by the `dataframe` crate.
#[derive(Debug)]
pub enum DfError {
    /// An error surfaced by the underlying `polars` engine.
    Polars(polars::error::PolarsError),
    /// An I/O error while reading/writing a data file.
    Io(std::io::Error),
    /// Any other error condition.
    Other(String),
}

impl std::fmt::Display for DfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DfError::Polars(e) => write!(f, "polars error: {e}"),
            DfError::Io(e) => write!(f, "io error: {e}"),
            DfError::Other(msg) => write!(f, "error: {msg}"),
        }
    }
}

impl std::error::Error for DfError {}

impl From<polars::error::PolarsError> for DfError {
    fn from(e: polars::error::PolarsError) -> Self {
        DfError::Polars(e)
    }
}

impl From<std::io::Error> for DfError {
    fn from(e: std::io::Error) -> Self {
        DfError::Io(e)
    }
}

/// Result type used throughout the crate.
pub type DfResult<T> = std::result::Result<T, DfError>;
