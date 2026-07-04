use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct FileManager {
    files: Arc<DashMap<String, FileMetadata>>,
    bookmarks: Arc<DashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: u64,
    pub is_directory: bool,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            files: Arc::new(DashMap::new()),
            bookmarks: Arc::new(DashMap::new()),
        }
    }

    pub fn index_file(&self, metadata: FileMetadata) -> Result<()> {
        self.files.insert(metadata.path.clone(), metadata);
        Ok(())
    }

    pub fn search_files(&self, pattern: &str) -> Vec<FileMetadata> {
        self.files
            .iter()
            .filter(|f| f.value().name.to_lowercase().contains(&pattern.to_lowercase()))
            .map(|f| f.value().clone())
            .collect()
    }

    pub fn add_bookmark(&self, name: String, path: String) -> Result<()> {
        self.bookmarks.insert(name, path);
        Ok(())
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_manager() {
        let mgr = FileManager::new();
        let meta = FileMetadata {
            path: "/doc.txt".to_string(),
            name: "doc.txt".to_string(),
            size: 1024,
            modified: 1000,
            is_directory: false,
        };
        assert!(mgr.index_file(meta).is_ok());
        assert_eq!(mgr.file_count(), 1);
    }
}
