//! Intelligent file watcher for auto-compilation

use crate::Result;
use std::path::PathBuf;
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum WatchEvent {
    FileModified(PathBuf),
    FileCreated(PathBuf),
    FileDeleted(PathBuf),
    DirectoryChanged(PathBuf),
}

/// Intelligent file watcher
pub struct FileWatcher {
    watched_paths: Vec<PathBuf>,
    debounce_ms: u64,
    ignore_patterns: Vec<String>,
}

impl FileWatcher {
    /// Create new file watcher
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            watched_paths: Vec::new(),
            debounce_ms,
            ignore_patterns: vec![
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                ".git".to_string(),
                "node_modules".to_string(),
                ".cache".to_string(),
            ],
        }
    }

    /// Add path to watch
    pub fn watch(&mut self, path: PathBuf) {
        self.watched_paths.push(path);
    }

    /// Watch directory with debouncing
    pub async fn start(&self) -> Result<mpsc::Receiver<WatchEvent>> {
        let (_tx, rx) = mpsc::channel();

        // Stub: real implementation would use notify crate
        // to watch filesystem and emit events

        for path in &self.watched_paths {
            log::info!("Watching directory: {}", path.display());
        }

        Ok(rx)
    }

    /// Check if file should trigger rebuild
    fn should_rebuild(&self, path: &PathBuf) -> bool {
        let path_str = path.to_string_lossy();

        // Check ignore patterns
        for pattern in &self.ignore_patterns {
            if path_str.contains(pattern) {
                return false;
            }
        }

        // Check file extensions
        let extensions_to_watch = [
            "rs", "py", "go", "ts", "js", "java", "kt", "cs", "swift",
            "toml", "yaml", "json", "cpp", "h", "c", "rb", "php",
        ];

        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return extensions_to_watch.contains(&ext_str);
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new(500);
        assert_eq!(watcher.debounce_ms, 500);
    }

    #[test]
    fn test_should_rebuild() {
        let watcher = FileWatcher::new(500);

        assert!(watcher.should_rebuild(&PathBuf::from("main.rs")));
        assert!(watcher.should_rebuild(&PathBuf::from("app.py")));
        assert!(!watcher.should_rebuild(&PathBuf::from("target/debug/app")));
    }

    #[test]
    fn test_watch_path_addition() {
        let mut watcher = FileWatcher::new(500);
        watcher.watch(PathBuf::from("/project"));
        assert_eq!(watcher.watched_paths.len(), 1);
    }
}
