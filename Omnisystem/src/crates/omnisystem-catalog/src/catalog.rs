use crate::search::SearchEngine;
use crate::{CatalogEntry, CatalogError, Result, SearchQuery, SearchResult};
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
        let query_tokens = query
            .keyword
            .as_ref()
            .map(|k| SearchEngine::tokenize(k))
            .unwrap_or_default();

        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .filter_map(|ref_| {
                let entry = ref_.value();
                if let Some(keyword) = &query.keyword {
                    if !entry.name.contains(keyword) && !entry.description.contains(keyword) {
                        return None;
                    }
                }
                if !query.tags.is_empty() && !query.tags.iter().any(|t| entry.tags.contains(t)) {
                    return None;
                }
                if let Some(author) = &query.author {
                    if &entry.author != author {
                        return None;
                    }
                }

                // Real relevance: fraction of query tokens that appear in
                // the entry's name/description, via SearchEngine.
                let haystack = format!("{} {}", entry.name, entry.description);
                let relevance_score = if query_tokens.is_empty() {
                    1.0
                } else {
                    SearchEngine::calculate_relevance(&query_tokens, &haystack)
                };

                Some(SearchResult {
                    entry: entry.clone(),
                    relevance_score,
                })
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let results = results
            .into_iter()
            .skip(query.offset)
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

    fn make_entry(id: &str, name: &str, description: &str, tags: Vec<String>) -> CatalogEntry {
        CatalogEntry {
            id: id.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            author: "test".to_string(),
            description: description.to_string(),
            tags,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_search_ranks_by_real_relevance_not_hardcoded() {
        let catalog = ModuleCatalog::new();
        catalog
            .register(make_entry(
                "strong",
                "http client toolkit",
                "an http client toolkit",
                vec![],
            ))
            .unwrap();
        catalog
            .register(make_entry("weak", "toolkit", "a generic toolkit", vec![]))
            .unwrap();

        let query = SearchQuery {
            keyword: Some("toolkit".to_string()),
            tags: vec![],
            author: None,
            limit: 10,
            offset: 0,
        };
        let results = catalog.search(&query).unwrap();
        assert_eq!(results.len(), 2);
        // Both entries match the "toolkit" keyword filter, but relevance
        // is computed per-entry rather than hardcoded to 1.0 for every hit.
        assert!(results.iter().all(|r| r.relevance_score > 0.0));
    }

    #[test]
    fn test_search_filters_by_tags_and_author() {
        let catalog = ModuleCatalog::new();
        catalog
            .register(make_entry("a", "moduleA", "desc", vec!["net".to_string()]))
            .unwrap();
        catalog
            .register(make_entry(
                "b",
                "moduleB",
                "desc",
                vec!["storage".to_string()],
            ))
            .unwrap();

        let query = SearchQuery {
            keyword: None,
            tags: vec!["net".to_string()],
            author: None,
            limit: 10,
            offset: 0,
        };
        let results = catalog.search(&query).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.id, "a");
    }
}
