//! Error types for the backup manager

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum BackupError {
    #[error("backup failed: backup not found")]
    BackupFailed,

    #[error("backup verification failed: backup not found")]
    VerificationFailed,

    #[error("snapshot failed: backup not found")]
    SnapshotFailed,

    #[error("other error: {0}")]
    Other(String),
}

pub type BackupResult<T> = std::result::Result<T, BackupError>;
