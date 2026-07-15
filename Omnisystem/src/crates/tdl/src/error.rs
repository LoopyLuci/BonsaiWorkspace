//! Error types for the Training Data Library.

/// Errors produced by the `tdl` crate.
#[derive(Debug)]
pub enum TdlError {
    /// A quality score outside the valid `0.0..=1.0` range was supplied.
    InvalidQualityScore(f32),
    /// An unrecognised export format string was supplied.
    InvalidFormat(String),
    /// A version id/string could not be found.
    VersionNotFound(String),
    /// An error from the underlying SQLite database.
    Database(sqlx::Error),
    /// A JSON (de)serialisation error.
    Json(serde_json::Error),
    /// A timestamp failed to parse as RFC3339.
    DateParse(chrono::format::ParseError),
    /// A UUID failed to parse.
    Uuid(uuid::Error),
    /// A filesystem I/O error.
    Io(std::io::Error),
    /// An error from the Arrow in-memory format.
    Arrow(arrow::error::ArrowError),
    /// An error from the Parquet writer.
    Parquet(parquet::errors::ParquetError),
    /// Any other error condition.
    Other(String),
}

impl std::fmt::Display for TdlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TdlError::InvalidQualityScore(score) => {
                write!(f, "invalid quality score {score}: must be within 0.0..=1.0")
            }
            TdlError::InvalidFormat(fmt_str) => write!(f, "invalid export format: {fmt_str}"),
            TdlError::VersionNotFound(id) => write!(f, "version not found: {id}"),
            TdlError::Database(e) => write!(f, "database error: {e}"),
            TdlError::Json(e) => write!(f, "json error: {e}"),
            TdlError::DateParse(e) => write!(f, "date parse error: {e}"),
            TdlError::Uuid(e) => write!(f, "uuid parse error: {e}"),
            TdlError::Io(e) => write!(f, "io error: {e}"),
            TdlError::Arrow(e) => write!(f, "arrow error: {e}"),
            TdlError::Parquet(e) => write!(f, "parquet error: {e}"),
            TdlError::Other(msg) => write!(f, "error: {msg}"),
        }
    }
}

impl std::error::Error for TdlError {}

impl From<sqlx::Error> for TdlError {
    fn from(e: sqlx::Error) -> Self {
        TdlError::Database(e)
    }
}

impl From<serde_json::Error> for TdlError {
    fn from(e: serde_json::Error) -> Self {
        TdlError::Json(e)
    }
}

impl From<chrono::format::ParseError> for TdlError {
    fn from(e: chrono::format::ParseError) -> Self {
        TdlError::DateParse(e)
    }
}

impl From<uuid::Error> for TdlError {
    fn from(e: uuid::Error) -> Self {
        TdlError::Uuid(e)
    }
}

impl From<std::io::Error> for TdlError {
    fn from(e: std::io::Error) -> Self {
        TdlError::Io(e)
    }
}

impl From<arrow::error::ArrowError> for TdlError {
    fn from(e: arrow::error::ArrowError) -> Self {
        TdlError::Arrow(e)
    }
}

impl From<parquet::errors::ParquetError> for TdlError {
    fn from(e: parquet::errors::ParquetError) -> Self {
        TdlError::Parquet(e)
    }
}

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, TdlError>;
