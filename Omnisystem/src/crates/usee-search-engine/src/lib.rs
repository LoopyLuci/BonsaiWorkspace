use dashmap::DashMap;
use module_interfaces::{ModuleId, ModuleInfo};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tracing::{debug, info};
use universal_module_registry::ModuleRegistry;

pub struct SearchEngine {
    registry: Arc<ModuleRegistry>,
    name_index: Arc<BTreeMap<String, Vec<String>>>,
    tag_index: Arc<DashMap<String, Vec<String>>>,
    capability_index: Arc<DashMap<String, Vec<String>>>,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub module_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub relevance_score: f64,
    pub tags: Vec<String>,
}

impl SearchEngine {
    pub fn new(registry: Arc<ModuleRegistry>) -> Self {
        info!("Creating SearchEngine for {} modules", registry.count_modules());
        let mut name_index = BTreeMap::new();

        for module in registry.list_all_modules() {
            let name_lower = module.name.to_lowercase();
            name_index.entry(name_lower).or_insert_with(Vec::new).push(module.id.0.clone());
        }

        let mut tag_index = DashMap::new();
        for module in registry.list_all_modules() {
            for tag in &module.tags {
                tag_index.entry(tag.clone()).or_insert_with(Vec::new).push(module.id.0.clone());
            }
        }

        let mut capability_index = DashMap::new();
        for module in registry.list_all_modules() {
            for cap in &module.capabilities {
                capability_index.entry(cap.name.clone()).or_insert_with(Vec::new).push(module.id.0.clone());
            }
        }

        Self {
            registry,
            name_index: Arc::new(name_index),
            tag_index: Arc::new(tag_index),
            capability_index: Arc::new(capability_index),
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        debug!("Searching for: {}", query);

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        if query_lower.contains("type:") {
            return self.search_by_type(query);
        }

        if query_lower.contains("capability:") {
            return self.search_by_capability(query);
        }

        if query_lower.contains("tag:") {
            return self.search_by_tag(query);
        }

        for module in self.registry.list_all_modules() {
            let name_match = module.name.to_lowercase().contains(&query_lower);
            let desc_match = module.description.to_lowercase().contains(&query_lower);

            if name_match || desc_match {
                let score = if name_match { 1.0 } else { 0.7 };
                results.push(SearchResult {
                    module_id: module.id.0.clone(),
                    name: module.name.clone(),
                    version: module.version.to_string_canonical(),
                    description: module.description.clone(),
                    relevance_score: score,
                    tags: module.tags.clone(),
                });
            }
        }

        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        Ok(results)
    }

    pub async fn search_prefix(&self, prefix: &str) -> Result<Vec<SearchResult>, String> {
        debug!("Prefix search for: {}", prefix);
        let prefix_lower = prefix.to_lowercase();
        let mut results = Vec::new();

        for module in self.registry.list_all_modules() {
            if module.name.to_lowercase().starts_with(&prefix_lower) {
                results.push(SearchResult {
                    module_id: module.id.0.clone(),
                    name: module.name.clone(),
                    version: module.version.to_string_canonical(),
                    description: module.description.clone(),
                    relevance_score: 0.95,
                    tags: module.tags.clone(),
                });
            }
        }

        Ok(results)
    }

    pub async fn search_by_tag(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        let tag = query.replace("tag:", "").trim().to_string();
        let mut results = Vec::new();

        if let Some(module_ids) = self.tag_index.get(&tag) {
            for module_id in module_ids.value() {
                if let Ok(module) = self.registry.get_module(module_id) {
                    results.push(SearchResult {
                        module_id: module.id.0.clone(),
                        name: module.name.clone(),
                        version: module.version.to_string_canonical(),
                        description: module.description.clone(),
                        relevance_score: 0.9,
                        tags: module.tags.clone(),
                    });
                }
            }
        }

        Ok(results)
    }

    pub async fn search_by_capability(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        let capability = query.replace("capability:", "").trim().to_string();
        let mut results = Vec::new();

        if let Some(module_ids) = self.capability_index.get(&capability) {
            for module_id in module_ids.value() {
                if let Ok(module) = self.registry.get_module(module_id) {
                    results.push(SearchResult {
                        module_id: module.id.0.clone(),
                        name: module.name.clone(),
                        version: module.version.to_string_canonical(),
                        description: module.description.clone(),
                        relevance_score: 0.95,
                        tags: module.tags.clone(),
                    });
                }
            }
        }

        Ok(results)
    }

    pub async fn search_by_type(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        let module_type = query.replace("type:", "").trim().to_string();
        let mut results = Vec::new();

        for module in self.registry.list_all_modules() {
            let type_name = format!("{:?}", module.id).to_lowercase();
            if type_name.contains(&module_type) {
                results.push(SearchResult {
                    module_id: module.id.0.clone(),
                    name: module.name.clone(),
                    version: module.version.to_string_canonical(),
                    description: module.description.clone(),
                    relevance_score: 0.9,
                    tags: module.tags.clone(),
                });
            }
        }

        Ok(results)
    }

    pub fn get_module(&self, module_id: &str) -> Result<ModuleInfo, String> {
        self.registry.get_module(module_id)
            .map_err(|e| e.to_string())
    }

    pub fn autocomplete(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        self.registry
            .list_all_modules()
            .iter()
            .filter(|m| m.name.to_lowercase().starts_with(&prefix_lower))
            .map(|m| m.name.clone())
            .take(10)
            .collect()
    }
}

impl Clone for SearchEngine {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            name_index: Arc::clone(&self.name_index),
            tag_index: Arc::clone(&self.tag_index),
            capability_index: Arc::clone(&self.capability_index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_engine_creation() {
        let registry = Arc::new(ModuleRegistry::new());
        let engine = SearchEngine::new(registry);
        let results = engine.search("test").await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_prefix_search() {
        let registry = Arc::new(ModuleRegistry::new());
        let engine = SearchEngine::new(registry);
        let results = engine.search_prefix("test").await.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_autocomplete() {
        let registry = Arc::new(ModuleRegistry::new());
        let engine = SearchEngine::new(registry);
        let suggestions = engine.autocomplete("test");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_search_result_creation() {
        let result = SearchResult {
            module_id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test module".to_string(),
            relevance_score: 0.95,
            tags: vec!["test".to_string()],
        };
        assert_eq!(result.relevance_score, 0.95);
    }

    #[test]
    fn test_search_engine_clone() {
        let registry = Arc::new(ModuleRegistry::new());
        let engine = SearchEngine::new(registry);
        let cloned = engine.clone();
        assert_eq!(engine.registry.count_modules(), cloned.registry.count_modules());
    }
}
