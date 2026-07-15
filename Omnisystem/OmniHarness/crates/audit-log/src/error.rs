//! Error types for the audit-log ledger.

#[derive(Debug, Clone)]
pub enum Error {
    /// SQLite-backed store failure (open, schema, query, or write).
    Db(String),
    /// (De)serialization of an event or snapshot failed.
    Serialization(String),
    /// Filesystem access failed (e.g. creating the database's parent directory).
    Io(String),
    /// Anything that doesn't fit the above.
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Db(msg) => write!(f, "audit-log store error: {}", msg),
            Error::Serialization(msg) => write!(f, "audit-log serialization error: {}", msg),
            Error::Io(msg) => write!(f, "audit-log io error: {}", msg),
            Error::Other(msg) => write!(f, "audit-log error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Db(e.to_string())
    }
}

impl From<tokio_rusqlite::Error> for Error {
    fn from(e: tokio_rusqlite::Error) -> Self {
        Error::Db(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

/// Result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
