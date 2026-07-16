//! Error types for the omnifile-explorer crate.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileError {
    /// No file is registered under the given path.
    FileNotFound(String),
    /// Catch-all for anything else.
    Other(String),
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::FileNotFound(path) => write!(f, "file not found: {}", path),
            FileError::Other(msg) => write!(f, "omnifile-explorer error: {}", msg),
        }
    }
}

impl std::error::Error for FileError {}

/// Result type used throughout the omnifile-explorer crate.
pub type Result<T> = std::result::Result<T, FileError>;

// Backwards-compatible alias (matches the original stub's naming).
pub type Error = FileError;
