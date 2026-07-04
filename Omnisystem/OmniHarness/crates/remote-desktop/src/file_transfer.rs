//! File transfer with CAS-based delta compression.
//!
//! Transfers files between peers using Content-Addressable Storage (CAS)
//! for deduplication and delta compression.

use crate::SessionId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during file transfer operations.
#[derive(Debug, Error)]
pub enum FileTransferError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Transfer failed: {reason}")]
    TransferFailed { reason: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Disk space exceeded")]
    DiskSpaceExceeded,

    #[error("Invalid path")]
    InvalidPath,
}

/// File transfer direction.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransferDirection {
    /// Upload file to remote.
    Upload,
    /// Download file from remote.
    Download,
    /// Bidirectional sync.
    Sync,
}

/// File metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path.
    pub path: PathBuf,

    /// File size in bytes.
    pub size: u64,

    /// Last modified timestamp (Unix seconds).
    pub modified: u64,

    /// File permissions (Unix mode).
    pub permissions: u32,

    /// SHA256 hash of file content.
    pub hash: String,

    /// Whether this is a directory.
    pub is_dir: bool,
}

impl FileMetadata {
    /// Create from file path (stub).
    pub fn from_path(path: &Path) -> Result<Self, FileTransferError> {
        let path_str = path
            .to_str()
            .ok_or(FileTransferError::InvalidPath)?
            .to_string();

        Ok(FileMetadata {
            path: path.to_path_buf(),
            size: 0,
            modified: 0,
            permissions: 0o644,
            hash: String::new(),
            is_dir: false,
        })
    }
}

/// File transfer progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    /// Total bytes to transfer.
    pub total_bytes: u64,

    /// Bytes transferred so far.
    pub bytes_transferred: u64,

    /// Transfer percentage (0-100).
    pub percent: f64,

    /// Current speed in MB/s.
    pub speed_mbs: f64,

    /// Estimated time remaining (seconds).
    pub eta_seconds: Option<u64>,

    /// Is transfer complete.
    pub complete: bool,
}

impl TransferProgress {
    /// Create a new transfer progress tracker.
    pub fn new(total_bytes: u64) -> Self {
        TransferProgress {
            total_bytes,
            bytes_transferred: 0,
            percent: 0.0,
            speed_mbs: 0.0,
            eta_seconds: None,
            complete: false,
        }
    }

    /// Update progress.
    pub fn update(&mut self, bytes_transferred: u64) {
        self.bytes_transferred = bytes_transferred.min(self.total_bytes);
        self.percent = (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0;
        self.complete = self.bytes_transferred >= self.total_bytes;
    }
}

/// File transfer service using CAS-based delta compression.
pub struct FileTransferService {
    /// Active transfers (SessionId -> TransferProgress).
    transfers: Arc<tokio::sync::RwLock<std::collections::HashMap<SessionId, TransferProgress>>>,

    /// CAS storage path.
    cas_path: PathBuf,
}

impl FileTransferService {
    /// Create a new FileTransferService.
    pub fn new() -> Self {
        FileTransferService {
            transfers: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            cas_path: PathBuf::from(".bonsai-remote-desktop-cas"),
        }
    }

    /// Start a file transfer.
    pub async fn start_transfer(
        &self,
        session_id: SessionId,
        source: &Path,
        destination: &Path,
        direction: TransferDirection,
    ) -> Result<(), FileTransferError> {
        let metadata = FileMetadata::from_path(source)?;

        let progress = TransferProgress::new(metadata.size);
        self.transfers.write().await.insert(session_id, progress);

        tracing::info!(
            "Started {} transfer from {:?} to {:?}",
            match direction {
                TransferDirection::Upload => "upload",
                TransferDirection::Download => "download",
                TransferDirection::Sync => "sync",
            },
            source,
            destination
        );

        Ok(())
    }

    /// Update transfer progress.
    pub async fn update_transfer(
        &self,
        session_id: SessionId,
        bytes_transferred: u64,
    ) -> Result<(), FileTransferError> {
        if let Some(progress) = self.transfers.write().await.get_mut(&session_id) {
            progress.update(bytes_transferred);
            Ok(())
        } else {
            Err(FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Get transfer progress.
    pub async fn get_progress(
        &self,
        session_id: SessionId,
    ) -> Result<TransferProgress, FileTransferError> {
        self.transfers
            .read()
            .await
            .get(&session_id)
            .cloned()
            .ok_or(FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
    }

    /// Complete a transfer.
    pub async fn complete_transfer(&self, session_id: SessionId) -> Result<(), FileTransferError> {
        if self.transfers.write().await.remove(&session_id).is_some() {
            tracing::info!("Completed transfer for session {}", session_id);
            Ok(())
        } else {
            Err(FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Cancel a transfer.
    pub async fn cancel_transfer(&self, session_id: SessionId) -> Result<(), FileTransferError> {
        if self.transfers.write().await.remove(&session_id).is_some() {
            tracing::info!("Cancelled transfer for session {}", session_id);
            Ok(())
        } else {
            Err(FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Get CAS hash for a file (stub).
    pub async fn get_file_hash(&self, path: &Path) -> Result<String, FileTransferError> {
        use sha2::{Sha256, Digest};

        let path_str = path
            .to_str()
            .ok_or(FileTransferError::InvalidPath)?;

        // Stub: return a mock hash
        let mut hasher = Sha256::new();
        hasher.update(path_str.as_bytes());
        let result = hasher.finalize();

        Ok(format!("{:x}", result))
    }

    /// List all active transfers.
    pub async fn list_transfers(&self) -> Vec<SessionId> {
        self.transfers.read().await.keys().cloned().collect()
    }
}

impl Default for FileTransferService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_transfer() {
        let service = FileTransferService::new();
        let session_id = SessionId::new();

        let result = service
            .start_transfer(
                session_id,
                Path::new("/tmp/source.txt"),
                Path::new("/tmp/dest.txt"),
                TransferDirection::Upload,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_progress() {
        let service = FileTransferService::new();
        let session_id = SessionId::new();

        service
            .start_transfer(
                session_id,
                Path::new("/tmp/source.txt"),
                Path::new("/tmp/dest.txt"),
                TransferDirection::Upload,
            )
            .await
            .unwrap();

        service.update_transfer(session_id, 500).await.unwrap();

        let progress = service.get_progress(session_id).await.unwrap();
        assert_eq!(progress.bytes_transferred, 500);
    }

    #[tokio::test]
    async fn test_complete_transfer() {
        let service = FileTransferService::new();
        let session_id = SessionId::new();

        service
            .start_transfer(
                session_id,
                Path::new("/tmp/source.txt"),
                Path::new("/tmp/dest.txt"),
                TransferDirection::Upload,
            )
            .await
            .unwrap();

        service.complete_transfer(session_id).await.unwrap();

        let result = service.get_progress(session_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_file_hash() {
        let service = FileTransferService::new();
        let hash = service.get_file_hash(Path::new("/tmp/test.txt")).await.unwrap();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_transfer_progress() {
        let mut progress = TransferProgress::new(1000);
        assert_eq!(progress.percent, 0.0);

        progress.update(500);
        assert_eq!(progress.percent, 50.0);
        assert!(!progress.complete);

        progress.update(1000);
        assert_eq!(progress.percent, 100.0);
        assert!(progress.complete);
    }
}
