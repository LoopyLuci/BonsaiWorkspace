//! Error types for app-manager-repository

#[derive(Debug)]
pub enum RepositoryError {
    /// Manifest failed validation
    InvalidManifest,
    /// Package data is corrupted or empty
    CorruptedPackage,
    /// Signature verification failed
    InvalidSignature,
    /// Requested resource not found
    NotFound(String),
    /// GitHub API/transport error
    GitHubError(String),
    /// Generic network error
    NetworkError(String),
    /// Local filesystem I/O error
    IoError(std::io::Error),
    /// Generic validation failure
    ValidationFailed(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::InvalidManifest => write!(f, "invalid manifest"),
            RepositoryError::CorruptedPackage => write!(f, "corrupted package"),
            RepositoryError::InvalidSignature => write!(f, "invalid signature"),
            RepositoryError::NotFound(what) => write!(f, "not found: {}", what),
            RepositoryError::GitHubError(msg) => write!(f, "GitHub error: {}", msg),
            RepositoryError::NetworkError(msg) => write!(f, "network error: {}", msg),
            RepositoryError::IoError(e) => write!(f, "I/O error: {}", e),
            RepositoryError::ValidationFailed(msg) => write!(f, "validation failed: {}", msg),
        }
    }
}

impl std::error::Error for RepositoryError {}

impl From<std::io::Error> for RepositoryError {
    fn from(e: std::io::Error) -> Self {
        RepositoryError::IoError(e)
    }
}

/// Result type
pub type Result<T> = std::result::Result<T, RepositoryError>;
