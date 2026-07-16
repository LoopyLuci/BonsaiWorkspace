use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct FileIndexer {
    index: Arc<DashMap<String, IndexEntry>>,
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub tags: Vec<String>,
}

impl FileIndexer {
    pub fn new() -> Self {
        Self {
            index: Arc::new(DashMap::new()),
        }
    }

    pub fn index_file(&self, entry: IndexEntry) -> Result<()> {
        self.index.insert(entry.path.clone(), entry);
        tracing::info!("File indexed");
        Ok(())
    }

    pub fn search_by_name(&self, pattern: &str) -> Vec<IndexEntry> {
        self.index
            .iter()
            .filter(|e| e.value().name.contains(pattern))
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn search_by_size(&self, min: u64, max: u64) -> Vec<IndexEntry> {
        self.index
            .iter()
            .filter(|e| e.value().size >= min && e.value().size <= max)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn index_count(&self) -> usize {
        self.index.len()
    }
}

impl Default for FileIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexer() {
        let indexer = FileIndexer::new();
        let entry = IndexEntry {
            path: "/file.txt".to_string(),
            name: "file.txt".to_string(),
            size: 1024,
            tags: vec!["document".to_string()],
        };
        assert!(indexer.index_file(entry).is_ok());
        assert_eq!(indexer.index_count(), 1);
    }

    #[test]
    fn test_search_by_name() {
        let indexer = FileIndexer::new();
        let entry = IndexEntry {
            path: "/document.pdf".to_string(),
            name: "document.pdf".to_string(),
            size: 5000,
            tags: vec![],
        };
        indexer.index_file(entry).unwrap();
        let results = indexer.search_by_name("document");
        assert_eq!(results.len(), 1);
    }
}
