//! Error types for the BKP package format

use std::path::PathBuf;

/// Errors that can occur while building, loading, or verifying BKP packages
#[derive(Debug, thiserror::Error)]
pub enum BkpError {
    /// A referenced file or path could not be found
    #[error("Not found: {0}")]
    NotFound(PathBuf),

    /// The package or a builder operation is invalid
    #[error("Invalid: {0}")]
    Invalid(String),

    /// The manifest could not be parsed or is malformed
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    /// Ed25519 signature verification failed
    #[error("Signature verification failed: {0}")]
    SignatureVerification(String),

    /// zstd compression/decompression failed
    #[error("zstd error: {0}")]
    Zstd(String),

    /// Underlying I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Zip archive error
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
}

impl BkpError {
    /// Construct a [`BkpError::Zstd`] variant
    pub fn zstd(msg: impl Into<String>) -> Self {
        BkpError::Zstd(msg.into())
    }
}

/// Result type used throughout the bkp crate
pub type BkpResult<T> = std::result::Result<T, BkpError>;

// Legacy generic error kept for compatibility with the stub `core` module.
#[derive(Debug, Clone)]
pub enum Error {
    /// Other error
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
