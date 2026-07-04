use crate::{VirtualFileSystem, FileMetadata, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct Explorer {
    vfs: Arc<VirtualFileSystem>,
    bookmarks: Arc<DashMap<String, String>>,
    current_path: Arc<std::sync::Mutex<String>>,
}

impl Explorer {
    pub fn new() -> Self {
        Self {
            vfs: Arc::new(VirtualFileSystem::new()),
            bookmarks: Arc::new(DashMap::new()),
            current_path: Arc::new(std::sync::Mutex::new("/".to_string())),
        }
    }

    pub fn create_file(&self, path: String, size: u64) -> Result<()> {
        let metadata = FileMetadata {
            path: path.clone(),
            size,
            modified: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            is_directory: false,
            permissions: 0o644,
        };
        self.vfs.create_file(path, metadata)
    }

    pub fn navigate(&self, path: String) {
        *self.current_path.lock().unwrap() = path;
    }

    pub fn get_current_path(&self) -> String {
        self.current_path.lock().unwrap().clone()
    }

    pub fn add_bookmark(&self, name: String, path: String) -> Result<()> {
        self.bookmarks.insert(name, path);
        Ok(())
    }

    pub fn file_count(&self) -> usize {
        self.vfs.file_count()
    }

    pub fn bookmark_count(&self) -> usize {
        self.bookmarks.len()
    }
}

impl Default for Explorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explorer() {
        let explorer = Explorer::new();
        assert!(explorer.create_file("/file.txt".to_string(), 1024).is_ok());
        assert_eq!(explorer.file_count(), 1);
    }

    #[test]
    fn test_navigation() {
        let explorer = Explorer::new();
        explorer.navigate("/home".to_string());
        assert_eq!(explorer.get_current_path(), "/home");
    }

    #[test]
    fn test_bookmarks() {
        let explorer = Explorer::new();
        assert!(explorer.add_bookmark("home".to_string(), "/home".to_string()).is_ok());
        assert_eq!(explorer.bookmark_count(), 1);
    }
}
