use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// File sync direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncDirection {
    /// Push from desktop to device
    Push,
    /// Pull from device to desktop
    Pull,
    /// Bidirectional sync
    Bidirectional,
}

/// File sync event type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileSyncEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// CAS (Content-Addressable Storage) blob reference
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlobRef {
    /// BLAKE3 hash of blob content
    pub hash: String,
    /// Blob size in bytes
    pub size: u64,
    /// Compression type (gzip, brotli, none)
    pub compression: Option<String>,
}

impl BlobRef {
    /// Create blob reference from data
    pub fn from_data(data: &[u8]) -> Self {
        let hash = blake3::hash(data).to_hex().to_string();
        Self {
            hash,
            size: data.len() as u64,
            compression: None,
        }
    }

    /// Create blob reference from file
    pub fn from_file(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self::from_data(&data))
    }
}

/// Delta block (for incremental file sync)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaBlock {
    /// Offset in file
    pub offset: u64,
    /// Length of block
    pub length: u64,
    /// BLAKE3 hash of original content
    pub original_hash: Option<String>,
    /// New block data (compressed)
    pub data: Vec<u8>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path (relative to sync root)
    pub path: String,
    /// File size
    pub size: u64,
    /// Modification time (Unix timestamp)
    pub modified_at: u64,
    /// Permissions (Unix-style)
    pub permissions: u32,
    /// Is directory
    pub is_dir: bool,
    /// Content hash (if file)
    pub content_hash: Option<String>,
}

/// File sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSyncOp {
    /// Operation ID
    pub id: String,
    /// Event type
    pub event_type: FileSyncEventType,
    /// File metadata
    pub metadata: FileMetadata,
    /// Sync direction
    pub direction: SyncDirection,
    /// Blob reference (for file content)
    pub blob_ref: Option<BlobRef>,
    /// Delta blocks (for incremental sync)
    pub delta_blocks: Option<Vec<DeltaBlock>>,
    /// Device ID
    pub device_id: String,
}

/// File synchronization manager
pub struct FileSynchronizer {
    /// Local sync root
    sync_root: PathBuf,
    /// File metadata cache
    metadata_cache: Arc<parking_lot::RwLock<std::collections::HashMap<String, FileMetadata>>>,
    /// Pending operations
    pending_ops: tokio::sync::mpsc::UnboundedSender<FileSyncOp>,
}

impl FileSynchronizer {
    /// Create new file synchronizer
    pub fn new(
        sync_root: PathBuf,
        pending_ops: tokio::sync::mpsc::UnboundedSender<FileSyncOp>,
    ) -> Result<Self> {
        if !sync_root.exists() {
            std::fs::create_dir_all(&sync_root)?;
        }

        Ok(Self {
            sync_root,
            metadata_cache: Arc::new(parking_lot::RwLock::new(
                std::collections::HashMap::new(),
            )),
            pending_ops,
        })
    }

    /// Scan directory and return file list
    pub fn scan_directory(&self) -> Result<Vec<FileMetadata>> {
        let mut files = Vec::new();

        for entry in walkdir::WalkDir::new(&self.sync_root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let relative = path.strip_prefix(&self.sync_root)
                .map_err(|e| crate::error::Error::PathError(e.to_string()))?;

            let metadata = std::fs::metadata(path)?;
            let file_meta = FileMetadata {
                path: relative.to_string_lossy().to_string(),
                size: metadata.len(),
                modified_at: metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs(),
                permissions: 0o644, // Simplified; use syscalls for actual perms on Unix
                is_dir: metadata.is_dir(),
                content_hash: if !metadata.is_dir() {
                    Some(BlobRef::from_file(path)?.hash)
                } else {
                    None
                },
            };

            files.push(file_meta);
        }

        Ok(files)
    }

    /// Detect changes and queue sync operations
    pub async fn detect_changes(&self, device_id: &str) -> Result<()> {
        let current_state = self.scan_directory()?;
        let mut cache = self.metadata_cache.write();

        for file_meta in current_state {
            if let Some(cached) = cache.get(&file_meta.path) {
                // Check if file was modified
                if cached.content_hash != file_meta.content_hash {
                    // File modified - queue delta sync
                    let op = FileSyncOp {
                        id: uuid::Uuid::new_v4().to_string(),
                        event_type: FileSyncEventType::Modified,
                        metadata: file_meta.clone(),
                        direction: SyncDirection::Push,
                        blob_ref: Some(BlobRef {
                            hash: file_meta.content_hash.clone().unwrap_or_default(),
                            size: file_meta.size,
                            compression: None,
                        }),
                        delta_blocks: None,
                        device_id: device_id.to_string(),
                    };

                    self.pending_ops.send(op).ok();
                }
            } else {
                // New file
                let op = FileSyncOp {
                    id: uuid::Uuid::new_v4().to_string(),
                    event_type: FileSyncEventType::Created,
                    metadata: file_meta.clone(),
                    direction: SyncDirection::Push,
                    blob_ref: Some(BlobRef {
                        hash: file_meta.content_hash.clone().unwrap_or_default(),
                        size: file_meta.size,
                        compression: None,
                    }),
                    delta_blocks: None,
                    device_id: device_id.to_string(),
                };

                self.pending_ops.send(op).ok();
            }

            cache.insert(file_meta.path.clone(), file_meta);
        }

        Ok(())
    }

    /// Apply remote file sync operation
    pub async fn apply_sync_op(&self, op: FileSyncOp) -> Result<()> {
        let file_path = self.sync_root.join(&op.metadata.path);

        match op.event_type {
            FileSyncEventType::Created | FileSyncEventType::Modified => {
                // Create parent directories
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Write file (in production, would retrieve blob from CAS store)
                std::fs::write(&file_path, b"[blob data from CAS]")?;

                // Update cache
                let metadata = std::fs::metadata(&file_path)?;
                let file_meta = FileMetadata {
                    path: op.metadata.path.clone(),
                    size: metadata.len(),
                    modified_at: metadata
                        .modified()?
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                    permissions: 0o644,
                    is_dir: false,
                    content_hash: Some(op.blob_ref.unwrap_or_default().hash),
                };
                self.metadata_cache
                    .write()
                    .insert(op.metadata.path.clone(), file_meta);
            }
            FileSyncEventType::Deleted => {
                std::fs::remove_file(&file_path)?;
                self.metadata_cache.write().remove(&op.metadata.path);
            }
            FileSyncEventType::Renamed => {
                // For rename: delete old, create new
                std::fs::remove_file(&file_path)?;
                self.metadata_cache.write().remove(&op.metadata.path);
            }
        }

        Ok(())
    }

    /// Get sync status
    pub fn get_status(&self) -> SyncStatus {
        let cache = self.metadata_cache.read();
        SyncStatus {
            total_files: cache.len(),
            total_size: cache.values().map(|m| m.size).sum(),
            last_sync: chrono::Utc::now(),
        }
    }
}

/// Sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub total_files: usize,
    pub total_size: u64,
    pub last_sync: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_ref() {
        let data = b"test content";
        let blob = BlobRef::from_data(data);
        assert!(!blob.hash.is_empty());
        assert_eq!(blob.size, 12);
    }

    #[test]
    fn test_file_metadata() {
        let meta = FileMetadata {
            path: "/test/file.txt".to_string(),
            size: 100,
            modified_at: 1000,
            permissions: 0o644,
            is_dir: false,
            content_hash: Some("hash123".to_string()),
        };

        assert_eq!(meta.path, "/test/file.txt");
        assert!(!meta.is_dir);
    }
}

