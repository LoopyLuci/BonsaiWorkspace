use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct VirtualFileSystem {
    files: Arc<DashMap<String, FileMetadata>>,
    directories: Arc<DashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub modified: u64,
    pub is_directory: bool,
    pub permissions: u32,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        Self {
            files: Arc::new(DashMap::new()),
            directories: Arc::new(DashMap::new()),
        }
    }

    pub fn create_file(&self, path: String, metadata: FileMetadata) -> Result<()> {
        self.files.insert(path, metadata);
        tracing::info!("File created");
        Ok(())
    }

    pub fn get_file(&self, path: &str) -> Result<FileMetadata> {
        self.files
            .get(path)
            .map(|m| m.value().clone())
            .ok_or_else(|| crate::FileError::FileNotFound(path.to_string()))
    }

    pub fn list_directory(&self, dir: &str) -> Option<Vec<String>> {
        self.directories.get(dir).map(|d| d.value().clone())
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for VirtualFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem() {
        let vfs = VirtualFileSystem::new();
        let meta = FileMetadata {
            path: "/file.txt".to_string(),
            size: 1024,
            modified: 1000,
            is_directory: false,
            permissions: 0o644,
        };
        assert!(vfs.create_file("/file.txt".to_string(), meta).is_ok());
        assert_eq!(vfs.file_count(), 1);
    }
}
