//! Error types for runtime provisioning

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProvisionerError {
    #[error("Runtime not found: {0}")]
    RuntimeNotFound(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Version not available: {0}")]
    VersionNotAvailable(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),
}

pub type Result<T> = std::result::Result<T, ProvisionerError>;
