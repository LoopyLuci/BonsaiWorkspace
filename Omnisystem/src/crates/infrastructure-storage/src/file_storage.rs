use crate::{
    DirectoryListing, FileContent, FileMetadata, FilePath, FileStorage, StorageError,
    StorageResult,
};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct InMemoryFileStorage {
    files: Arc<DashMap<String, FileContent>>,
    directories: Arc<DashMap<String, bool>>,
}

impl InMemoryFileStorage {
    pub fn new() -> Self {
        let storage = Self {
            files: Arc::new(DashMap::new()),
            directories: Arc::new(DashMap::new()),
        };
        // Create root directory
        storage.directories.insert("/".to_string(), true);
        storage
    }

    fn calculate_checksum(data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn get_parent_path(path: &str) -> String {
        if let Some(last_slash) = path.rfind('/') {
            if last_slash == 0 {
                "/".to_string()
            } else {
                path[..last_slash].to_string()
            }
        } else {
            "/".to_string()
        }
    }
}

impl Default for InMemoryFileStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileStorage for InMemoryFileStorage {
    async fn create_file(
        &self,
        path: FilePath,
        data: Vec<u8>,
    ) -> StorageResult<FileMetadata> {
        if self.files.contains_key(&path.0) {
            return Err(StorageError::InvalidOffset(
                "File already exists".to_string(),
            ));
        }

        let checksum = Self::calculate_checksum(&data);
        let size = data.len() as u64;
        let now = Utc::now();

        let metadata = FileMetadata {
            path: path.clone(),
            size,
            created_at: now,
            modified_at: now,
            is_directory: false,
            permissions: 0o644,
            owner: "root".to_string(),
            group: "root".to_string(),
            checksum: Some(checksum),
        };

        self.files.insert(
            path.0.clone(),
            FileContent {
                metadata: metadata.clone(),
                data,
            },
        );

        Ok(metadata)
    }

    async fn read_file(&self, path: &FilePath) -> StorageResult<FileContent> {
        self.files
            .get(&path.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| StorageError::ObjectNotFound(path.0.clone()))
    }

    async fn delete_file(&self, path: &FilePath) -> StorageResult<()> {
        self.files.remove(&path.0);
        Ok(())
    }

    async fn list_directory(&self, path: &FilePath) -> StorageResult<DirectoryListing> {
        if !self.directories.contains_key(&path.0) && path.0 != "/" {
            return Err(StorageError::ObjectNotFound(
                format!("Directory not found: {}", path.0),
            ));
        }

        let prefix = if path.0.ends_with('/') {
            path.0.clone()
        } else {
            format!("{}/", path.0)
        };

        let mut entries = Vec::new();
        let mut total_size = 0u64;

        for file_entry in self.files.iter() {
            if file_entry.key().starts_with(&prefix) {
                let meta = &file_entry.value().metadata;
                entries.push(meta.clone());
                total_size += meta.size;
            }
        }

        Ok(DirectoryListing {
            path: path.clone(),
            entries,
            total_size,
        })
    }

    async fn create_directory(&self, path: FilePath) -> StorageResult<FileMetadata> {
        if self.directories.contains_key(&path.0) {
            return Err(StorageError::PermissionDenied(
                "Directory already exists".to_string(),
            ));
        }

        self.directories.insert(path.0.clone(), true);

        let now = Utc::now();
        Ok(FileMetadata {
            path,
            size: 0,
            created_at: now,
            modified_at: now,
            is_directory: true,
            permissions: 0o755,
            owner: "root".to_string(),
            group: "root".to_string(),
            checksum: None,
        })
    }

    async fn copy_file(
        &self,
        source: &FilePath,
        destination: FilePath,
    ) -> StorageResult<FileMetadata> {
        let source_file = self.read_file(source).await?;
        self.create_file(destination, source_file.data).await
    }

    async fn move_file(
        &self,
        source: &FilePath,
        destination: FilePath,
    ) -> StorageResult<FileMetadata> {
        let file = self.read_file(source).await?;
        self.delete_file(source).await?;
        self.create_file(destination, file.data).await
    }

    async fn set_permissions(
        &self,
        path: &FilePath,
        permissions: u32,
    ) -> StorageResult<FileMetadata> {
        if let Some(mut entry) = self.files.get_mut(&path.0) {
            entry.metadata.permissions = permissions;
            Ok(entry.metadata.clone())
        } else {
            Err(StorageError::ObjectNotFound(path.0.clone()))
        }
    }

    async fn get_file_metadata(&self, path: &FilePath) -> StorageResult<FileMetadata> {
        self.files
            .get(&path.0)
            .map(|entry| entry.value().metadata.clone())
            .ok_or_else(|| StorageError::ObjectNotFound(path.0.clone()))
    }

    async fn append_file(
        &self,
        path: &FilePath,
        data: Vec<u8>,
    ) -> StorageResult<FileMetadata> {
        if let Some(mut entry) = self.files.get_mut(&path.0) {
            entry.data.extend_from_slice(&data);
            entry.metadata.size = entry.data.len() as u64;
            entry.metadata.modified_at = Utc::now();
            entry.metadata.checksum = Some(Self::calculate_checksum(&entry.data));
            Ok(entry.metadata.clone())
        } else {
            Err(StorageError::ObjectNotFound(path.0.clone()))
        }
    }

    async fn truncate_file(
        &self,
        path: &FilePath,
        size: u64,
    ) -> StorageResult<FileMetadata> {
        if let Some(mut entry) = self.files.get_mut(&path.0) {
            entry.data.truncate(size as usize);
            entry.metadata.size = size;
            entry.metadata.modified_at = Utc::now();
            entry.metadata.checksum = Some(Self::calculate_checksum(&entry.data));
            Ok(entry.metadata.clone())
        } else {
            Err(StorageError::ObjectNotFound(path.0.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_read_file() {
        let storage = InMemoryFileStorage::new();
        let path = FilePath("/data/file.txt".to_string());
        let data = b"Hello, World!".to_vec();

        storage
            .create_file(path.clone(), data.clone())
            .await
            .unwrap();

        let file = storage.read_file(&path).await.unwrap();
        assert_eq!(file.data, data);
        assert_eq!(file.metadata.size, 13);
    }

    #[tokio::test]
    async fn test_delete_file() {
        let storage = InMemoryFileStorage::new();
        let path = FilePath("/data/file.txt".to_string());

        storage
            .create_file(path.clone(), b"test".to_vec())
            .await
            .unwrap();

        storage.delete_file(&path).await.unwrap();

        let result = storage.read_file(&path).await;
        assert!(matches!(result, Err(StorageError::ObjectNotFound(_))));
    }

    #[tokio::test]
    async fn test_copy_file() {
        let storage = InMemoryFileStorage::new();
        let source = FilePath("/source.txt".to_string());
        let dest = FilePath("/dest.txt".to_string());
        let data = b"Copy me".to_vec();

        storage
            .create_file(source.clone(), data.clone())
            .await
            .unwrap();

        storage
            .copy_file(&source, dest.clone())
            .await
            .unwrap();

        let copied = storage.read_file(&dest).await.unwrap();
        assert_eq!(copied.data, data);
    }

    #[tokio::test]
    async fn test_move_file() {
        let storage = InMemoryFileStorage::new();
        let source = FilePath("/source.txt".to_string());
        let dest = FilePath("/moved.txt".to_string());

        storage
            .create_file(source.clone(), b"move me".to_vec())
            .await
            .unwrap();

        storage
            .move_file(&source, dest.clone())
            .await
            .unwrap();

        assert!(storage.read_file(&source).await.is_err());
        assert!(storage.read_file(&dest).await.is_ok());
    }

    #[tokio::test]
    async fn test_append_file() {
        let storage = InMemoryFileStorage::new();
        let path = FilePath("/file.txt".to_string());

        storage
            .create_file(path.clone(), b"Hello".to_vec())
            .await
            .unwrap();

        storage
            .append_file(&path, b" World".to_vec())
            .await
            .unwrap();

        let file = storage.read_file(&path).await.unwrap();
        assert_eq!(file.data, b"Hello World");
    }

    #[tokio::test]
    async fn test_truncate_file() {
        let storage = InMemoryFileStorage::new();
        let path = FilePath("/file.txt".to_string());

        storage
            .create_file(path.clone(), b"Hello World".to_vec())
            .await
            .unwrap();

        storage.truncate_file(&path, 5).await.unwrap();

        let file = storage.read_file(&path).await.unwrap();
        assert_eq!(file.data, b"Hello");
    }

    #[tokio::test]
    async fn test_list_directory() {
        let storage = InMemoryFileStorage::new();

        storage
            .create_file(FilePath("/file1.txt".to_string()), b"1".to_vec())
            .await
            .unwrap();
        storage
            .create_file(FilePath("/file2.txt".to_string()), b"2".to_vec())
            .await
            .unwrap();

        let listing = storage.list_directory(&FilePath("/".to_string())).await.unwrap();
        assert_eq!(listing.entries.len(), 2);
    }

    #[tokio::test]
    async fn test_set_permissions() {
        let storage = InMemoryFileStorage::new();
        let path = FilePath("/file.txt".to_string());

        storage
            .create_file(path.clone(), b"test".to_vec())
            .await
            .unwrap();

        storage.set_permissions(&path, 0o755).await.unwrap();

        let meta = storage.get_file_metadata(&path).await.unwrap();
        assert_eq!(meta.permissions, 0o755);
    }
}
