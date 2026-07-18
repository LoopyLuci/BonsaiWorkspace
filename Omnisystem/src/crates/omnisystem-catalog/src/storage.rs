use crate::{CatalogEntry, CatalogError, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub trait CatalogStorage: Send + Sync {
    fn save(&self, entry: &CatalogEntry) -> Result<()>;
    fn load(&self, id: &str) -> Result<CatalogEntry>;
    fn delete(&self, id: &str) -> Result<()>;
    fn list_all(&self) -> Result<Vec<CatalogEntry>>;
}

/// A real (if non-persistent) in-memory implementation of [`CatalogStorage`].
#[derive(Default)]
pub struct MemoryCatalogStorage {
    entries: Arc<DashMap<String, CatalogEntry>>,
}

impl MemoryCatalogStorage {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
        }
    }
}

impl CatalogStorage for MemoryCatalogStorage {
    fn save(&self, entry: &CatalogEntry) -> Result<()> {
        self.entries.insert(entry.id.clone(), entry.clone());
        Ok(())
    }

    fn load(&self, id: &str) -> Result<CatalogEntry> {
        self.entries
            .get(id)
            .map(|e| e.value().clone())
            .ok_or_else(|| CatalogError::NotFound(id.to_string()))
    }

    fn delete(&self, id: &str) -> Result<()> {
        self.entries
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| CatalogError::NotFound(id.to_string()))
    }

    fn list_all(&self) -> Result<Vec<CatalogEntry>> {
        Ok(self.entries.iter().map(|e| e.value().clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_entry(id: &str) -> CatalogEntry {
        CatalogEntry {
            id: id.to_string(),
            name: id.to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            description: "test entry".to_string(),
            tags: vec![],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_memory_storage_save_load_delete_roundtrip() {
        let storage = MemoryCatalogStorage::new();
        let entry = make_entry("test");

        storage.save(&entry).unwrap();
        let loaded = storage.load("test").unwrap();
        assert_eq!(loaded.id, "test");

        storage.delete("test").unwrap();
        assert!(storage.load("test").is_err());
    }

    #[test]
    fn test_memory_storage_delete_missing_errors() {
        let storage = MemoryCatalogStorage::new();
        assert!(storage.delete("missing").is_err());
    }

    #[test]
    fn test_memory_storage_list_all_reflects_saved_entries() {
        let storage = MemoryCatalogStorage::new();
        storage.save(&make_entry("a")).unwrap();
        storage.save(&make_entry("b")).unwrap();
        assert_eq!(storage.list_all().unwrap().len(), 2);
    }
}
