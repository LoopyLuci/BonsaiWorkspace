//! Error types

/// ICDS error type
#[derive(Debug, Clone)]
pub enum Error {
    /// Storage backend failure
    Storage(String),
    /// Index failure
    Index(String),
    /// System clock error
    Time(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Storage(msg) => write!(f, "storage error: {}", msg),
            Error::Index(msg) => write!(f, "index error: {}", msg),
            Error::Time(msg) => write!(f, "time error: {}", msg),
            Error::Other(msg) => write!(f, "error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::time::SystemTimeError> for Error {
    fn from(err: std::time::SystemTimeError) -> Self {
        Error::Time(err.to_string())
    }
}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
