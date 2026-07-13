use crate::{FileMetadata, FileOperations, FileType, FtpError, FtpResult, SessionId, TransferTracker, TransferInfo};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct FileTransferHandler {
    active_transfers: Arc<DashMap<String, TransferInfo>>,
    transfer_history: Arc<DashMap<String, Vec<TransferInfo>>>,
}

impl FileTransferHandler {
    pub fn new() -> Self {
        Self {
            active_transfers: Arc::new(DashMap::new()),
            transfer_history: Arc::new(DashMap::new()),
        }
    }

    pub fn active_transfer_count(&self) -> usize {
        self.active_transfers.len()
    }

    pub fn history_count(&self, session_id: &SessionId) -> usize {
        self.transfer_history
            .get(&session_id.0.to_string())
            .map(|entry| entry.len())
            .unwrap_or(0)
    }
}

impl Default for FileTransferHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransferTracker for FileTransferHandler {
    async fn start_transfer(&self, transfer: TransferInfo) -> FtpResult<()> {
        let key = format!("{}-{}", transfer.session_id.0, transfer.remote_path);
        self.active_transfers.insert(key, transfer);
        Ok(())
    }

    async fn update_transfer(&self, transfer: TransferInfo) -> FtpResult<()> {
        let key = format!("{}-{}", transfer.session_id.0, transfer.remote_path);
        self.active_transfers.insert(key, transfer);
        Ok(())
    }

    async fn complete_transfer(&self, transfer: TransferInfo) -> FtpResult<()> {
        let key = format!("{}-{}", transfer.session_id.0, transfer.remote_path);

        if let Some((_, completed)) = self.active_transfers.remove(&key) {
            let session_key = transfer.session_id.0.to_string();

            self.transfer_history
                .entry(session_key)
                .or_insert_with(Vec::new)
                .push(completed);
        }

        Ok(())
    }

    async fn get_active_transfers(&self, session_id: &SessionId) -> FtpResult<Vec<TransferInfo>> {
        let transfers: Vec<TransferInfo> = self
            .active_transfers
            .iter()
            .filter(|entry| entry.value().session_id == *session_id)
            .map(|entry| entry.value().clone())
            .collect();

        Ok(transfers)
    }

    async fn get_transfer_history(&self, _user_id: &crate::UserId) -> FtpResult<Vec<TransferInfo>> {
        let mut all_transfers = Vec::new();

        for entry in self.transfer_history.iter() {
            all_transfers.extend(entry.value().clone());
        }

        Ok(all_transfers)
    }
}

pub struct DefaultFileOperations {
    file_metadata: Arc<DashMap<String, FileMetadata>>,
}

impl DefaultFileOperations {
    pub fn new() -> Self {
        Self {
            file_metadata: Arc::new(DashMap::new()),
        }
    }

    pub fn file_count(&self) -> usize {
        self.file_metadata.len()
    }
}

impl Default for DefaultFileOperations {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileOperations for DefaultFileOperations {
    async fn upload_file(
        &self,
        _session_id: &SessionId,
        remote_path: &str,
        data: Vec<u8>,
    ) -> FtpResult<u64> {
        let metadata = FileMetadata {
            path: remote_path.to_string(),
            file_type: FileType::File,
            size: data.len() as u64,
            modified: Utc::now(),
            permissions: 0o644,
            owner: "system".to_string(),
        };

        self.file_metadata.insert(remote_path.to_string(), metadata);
        Ok(data.len() as u64)
    }

    async fn download_file(&self, _session_id: &SessionId, remote_path: &str) -> FtpResult<Vec<u8>> {
        if !self.file_metadata.contains_key(remote_path) {
            return Err(FtpError::FileNotFound(remote_path.to_string()));
        }

        Ok(vec![0u8; 1024])
    }

    async fn delete_file(&self, _session_id: &SessionId, remote_path: &str) -> FtpResult<()> {
        if self.file_metadata.remove(remote_path).is_some() {
            Ok(())
        } else {
            Err(FtpError::FileNotFound(remote_path.to_string()))
        }
    }

    async fn list_directory(
        &self,
        _session_id: &SessionId,
        path: &str,
    ) -> FtpResult<Vec<FileMetadata>> {
        let files: Vec<FileMetadata> = self
            .file_metadata
            .iter()
            .filter(|entry| entry.key().starts_with(path))
            .map(|entry| entry.value().clone())
            .collect();

        Ok(files)
    }

    async fn create_directory(&self, _session_id: &SessionId, path: &str) -> FtpResult<()> {
        let metadata = FileMetadata {
            path: path.to_string(),
            file_type: FileType::Directory,
            size: 0,
            modified: Utc::now(),
            permissions: 0o755,
            owner: "system".to_string(),
        };

        self.file_metadata.insert(path.to_string(), metadata);
        Ok(())
    }

    async fn delete_directory(&self, _session_id: &SessionId, path: &str) -> FtpResult<()> {
        if self.file_metadata.remove(path).is_some() {
            Ok(())
        } else {
            Err(FtpError::DirectoryNotFound(path.to_string()))
        }
    }

    async fn get_file_metadata(
        &self,
        _session_id: &SessionId,
        path: &str,
    ) -> FtpResult<FileMetadata> {
        self.file_metadata
            .get(path)
            .map(|entry| entry.clone())
            .ok_or_else(|| FtpError::FileNotFound(path.to_string()))
    }

    async fn rename_file(
        &self,
        _session_id: &SessionId,
        old_path: &str,
        new_path: &str,
    ) -> FtpResult<()> {
        if let Some((_, mut metadata)) = self.file_metadata.remove(old_path) {
            metadata.path = new_path.to_string();
            self.file_metadata.insert(new_path.to_string(), metadata);
            Ok(())
        } else {
            Err(FtpError::FileNotFound(old_path.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_start_transfer() {
        let handler = FileTransferHandler::new();
        let session_id = SessionId(Uuid::new_v4());

        let transfer = TransferInfo {
            session_id: session_id.clone(),
            remote_path: "/file.txt".to_string(),
            local_path: "/local/file.txt".to_string(),
            protocol: crate::Protocol::Ftp,
            bytes_transferred: 0,
            total_bytes: 1024,
            started_at: Utc::now(),
            completed_at: None,
        };

        handler.start_transfer(transfer).await.unwrap();
        assert_eq!(handler.active_transfer_count(), 1);
    }

    #[tokio::test]
    async fn test_upload_file() {
        let ops = DefaultFileOperations::new();
        let session_id = SessionId(Uuid::new_v4());

        let bytes = ops
            .upload_file(&session_id, "/file.txt", vec![0u8; 1024])
            .await
            .unwrap();

        assert_eq!(bytes, 1024);
        assert_eq!(ops.file_count(), 1);
    }

    #[tokio::test]
    async fn test_get_file_metadata() {
        let ops = DefaultFileOperations::new();
        let session_id = SessionId(Uuid::new_v4());

        ops.upload_file(&session_id, "/file.txt", vec![0u8; 1024])
            .await
            .unwrap();

        let metadata = ops.get_file_metadata(&session_id, "/file.txt").await.unwrap();
        assert_eq!(metadata.size, 1024);
    }

    #[tokio::test]
    async fn test_delete_file() {
        let ops = DefaultFileOperations::new();
        let session_id = SessionId(Uuid::new_v4());

        ops.upload_file(&session_id, "/file.txt", vec![0u8; 1024])
            .await
            .unwrap();

        assert_eq!(ops.file_count(), 1);

        ops.delete_file(&session_id, "/file.txt")
            .await
            .unwrap();

        assert_eq!(ops.file_count(), 0);
    }

    #[tokio::test]
    async fn test_create_directory() {
        let ops = DefaultFileOperations::new();
        let session_id = SessionId(Uuid::new_v4());

        ops.create_directory(&session_id, "/dir").await.unwrap();
        assert_eq!(ops.file_count(), 1);
    }

    #[tokio::test]
    async fn test_list_directory() {
        let ops = DefaultFileOperations::new();
        let session_id = SessionId(Uuid::new_v4());

        ops.upload_file(&session_id, "/file1.txt", vec![0u8; 100])
            .await
            .unwrap();

        ops.upload_file(&session_id, "/file2.txt", vec![0u8; 200])
            .await
            .unwrap();

        let files = ops.list_directory(&session_id, "/").await.unwrap();
        assert_eq!(files.len(), 2);
    }
}
