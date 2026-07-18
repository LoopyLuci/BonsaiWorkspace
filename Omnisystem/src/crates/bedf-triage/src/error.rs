//! Error types for bedf-triage

#[derive(Debug, Clone)]
pub enum Error {
    /// A crash report was submitted with an empty stack trace and cannot be
    /// hashed or triaged.
    EmptyStackTrace,
    /// Other error
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EmptyStackTrace => write!(f, "cannot triage a crash report with an empty stack trace"),
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
