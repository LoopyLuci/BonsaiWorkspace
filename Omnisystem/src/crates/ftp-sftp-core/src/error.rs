//! FTP/SFTP-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum FtpError {
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("file not found: {0}")]
    FileNotFound(String),
    #[error("directory not found: {0}")]
    DirectoryNotFound(String),
    #[error("configuration error: {0}")]
    ConfigurationError(String),
}

pub type FtpResult<T> = std::result::Result<T, FtpError>;
