use crate::{CatalogEntry, Result, CatalogError, SearchQuery, SearchResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ModuleCatalog {
    entries: Arc<DashMap<String, CatalogEntry>>,
    name_index: Arc<DashMap<String, Vec<String>>>,
}

impl ModuleCatalog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            name_index: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, entry: CatalogEntry) -> Result<()> {
        if self.entries.contains_key(&entry.id) {
            return Err(CatalogError::AlreadyExists(entry.id.clone()));
        }

        let entry_id = entry.id.clone();
        let name = entry.name.clone();
        self.entries.insert(entry_id.clone(), entry);

        self.name_index
            .entry(name)
            .or_insert_with(Vec::new)
            .push(entry_id);

        tracing::info!("Registered catalog entry");
        Ok(())
    }

    pub fn unregister(&self, id: &str) -> Result<()> {
        self.entries
            .remove(id)
            .ok_or_else(|| CatalogError::NotFound(id.to_string()))?;

        tracing::info!("Unregistered catalog entry");
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<CatalogEntry> {
        self.entries
            .get(id)
            .map(|ref_| ref_.value().clone())
            .ok_or_else(|| CatalogError::NotFound(id.to_string()))
    }

    pub fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let results: Vec<SearchResult> = self.entries
            .iter()
            .filter_map(|ref_| {
                let entry = ref_.value();
                if let Some(keyword) = &query.keyword {
                    if !entry.name.contains(keyword) && !entry.description.contains(keyword) {
                        return None;
                    }
                }
                Some(SearchResult {
                    entry: entry.clone(),
                    relevance_score: 1.0,
                })
            })
            .take(query.limit)
            .collect();

        Ok(results)
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn list_all(&self) -> Vec<CatalogEntry> {
        self.entries
            .iter()
            .map(|ref_| ref_.value().clone())
            .collect()
    }
}

impl Default for ModuleCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_catalog_new() {
        let catalog = ModuleCatalog::new();
        assert_eq!(catalog.count(), 0);
    }

    #[test]
    fn test_register_entry() {
        let catalog = ModuleCatalog::new();
        let entry = CatalogEntry {
            id: "test".to_string(),
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            description: "test".to_string(),
            tags: vec![],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        assert!(catalog.register(entry).is_ok());
        assert_eq!(catalog.count(), 1);
    }

    #[test]
    fn test_unregister_entry() {
        let catalog = ModuleCatalog::new();
        let entry = CatalogEntry {
            id: "test".to_string(),
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            description: "test".to_string(),
            tags: vec![],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        catalog.register(entry).unwrap();
        assert!(catalog.unregister("test").is_ok());
        assert_eq!(catalog.count(), 0);
    }
}
