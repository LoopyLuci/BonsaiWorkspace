use app_marketplace::AppMarketplace;
use dashmap::DashMap;
use module_interfaces::ModuleError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use usee_search_engine::SearchEngine;
use universal_module_registry::ModuleRegistry;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExplorerItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub item_type: ItemType,
    pub rating: f64,
    pub installs: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemType {
    Application,
    Module,
    Feature,
    Integration,
}

pub struct AppExplorer {
    registry: Arc<ModuleRegistry>,
    marketplace: Arc<AppMarketplace>,
    search_engine: Arc<SearchEngine>,
    categories: Arc<DashMap<String, Vec<ExplorerItem>>>,
    recent_items: Arc<DashMap<String, ExplorerItem>>,
}

impl AppExplorer {
    pub fn new(
        registry: Arc<ModuleRegistry>,
        marketplace: Arc<AppMarketplace>,
        search_engine: Arc<SearchEngine>,
    ) -> Self {
        info!("Creating AppExplorer");

        let mut categories = DashMap::new();
        categories.insert("productivity".to_string(), Vec::new());
        categories.insert("analytics".to_string(), Vec::new());
        categories.insert("infrastructure".to_string(), Vec::new());
        categories.insert("security".to_string(), Vec::new());
        categories.insert("compliance".to_string(), Vec::new());
        categories.insert("ai-ml".to_string(), Vec::new());
        categories.insert("data".to_string(), Vec::new());
        categories.insert("developer-tools".to_string(), Vec::new());

        Self {
            registry,
            marketplace,
            search_engine,
            categories: Arc::new(categories),
            recent_items: Arc::new(DashMap::new()),
        }
    }

    pub async fn browse_category(&self, category: &str) -> Result<Vec<ExplorerItem>, ModuleError> {
        debug!("Browsing category: {}", category);

        let mut items = Vec::new();

        for module in self.registry.list_all_modules() {
            if module.tags.contains(&category.to_string()) {
                items.push(ExplorerItem {
                    id: module.id.0.clone(),
                    name: module.name.clone(),
                    description: module.description.clone(),
                    category: category.to_string(),
                    item_type: ItemType::Module,
                    rating: 4.5,
                    installs: 100,
                });
            }
        }

        Ok(items)
    }

    pub fn get_categories(&self) -> Vec<String> {
        self.categories.iter().map(|entry| entry.key().clone()).collect()
    }

    pub async fn search(&self, query: &str) -> Result<Vec<ExplorerItem>, ModuleError> {
        debug!("Explorer search for: {}", query);

        let results = self.search_engine.search(query).await
            .map_err(|e| ModuleError::InternalError(e))?;

        let items = results.iter().map(|r| ExplorerItem {
            id: r.module_id.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            category: "search-result".to_string(),
            item_type: ItemType::Module,
            rating: r.relevance_score,
            installs: 0,
        }).collect();

        Ok(items)
    }

    pub fn add_to_recent(&self, item: ExplorerItem) {
        debug!("Adding to recent: {}", item.id);
        self.recent_items.insert(item.id.clone(), item);
    }

    pub fn get_recent(&self, limit: usize) -> Vec<ExplorerItem> {
        self.recent_items
            .iter()
            .take(limit)
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_trending(&self) -> Vec<ExplorerItem> {
        let mut items = Vec::new();

        for module in self.registry.list_all_modules().iter().take(10) {
            items.push(ExplorerItem {
                id: module.id.0.clone(),
                name: module.name.clone(),
                description: module.description.clone(),
                category: "trending".to_string(),
                item_type: ItemType::Module,
                rating: 4.8,
                installs: 10000,
            });
        }

        items.sort_by(|a, b| b.installs.cmp(&a.installs));
        items
    }
}

impl Clone for AppExplorer {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            marketplace: Arc::clone(&self.marketplace),
            search_engine: Arc::clone(&self.search_engine),
            categories: Arc::clone(&self.categories),
            recent_items: Arc::clone(&self.recent_items),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explorer_creation() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = Arc::new(universal_module_loader::ModuleLoader::new(registry.clone()));
        let search_engine = Arc::new(SearchEngine::new(registry.clone()));
        let marketplace = Arc::new(AppMarketplace::new(registry.clone(), loader, search_engine.clone()));
        let explorer = AppExplorer::new(registry, marketplace, search_engine);

        assert!(!explorer.get_categories().is_empty());
    }

    #[test]
    fn test_explorer_categories() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = Arc::new(universal_module_loader::ModuleLoader::new(registry.clone()));
        let search_engine = Arc::new(SearchEngine::new(registry.clone()));
        let marketplace = Arc::new(AppMarketplace::new(registry.clone(), loader, search_engine.clone()));
        let explorer = AppExplorer::new(registry, marketplace, search_engine);

        let categories = explorer.get_categories();
        assert!(categories.contains(&"productivity".to_string()));
    }

    #[test]
    fn test_explorer_item_creation() {
        let item = ExplorerItem {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test item".to_string(),
            category: "test".to_string(),
            item_type: ItemType::Module,
            rating: 4.5,
            installs: 100,
        };
        assert_eq!(item.rating, 4.5);
    }

    #[test]
    fn test_recent_items() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = Arc::new(universal_module_loader::ModuleLoader::new(registry.clone()));
        let search_engine = Arc::new(SearchEngine::new(registry.clone()));
        let marketplace = Arc::new(AppMarketplace::new(registry.clone(), loader, search_engine.clone()));
        let explorer = AppExplorer::new(registry, marketplace, search_engine);

        let item = ExplorerItem {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            category: "test".to_string(),
            item_type: ItemType::Module,
            rating: 4.5,
            installs: 100,
        };

        explorer.add_to_recent(item);
        let recent = explorer.get_recent(10);
        assert_eq!(recent.len(), 1);
    }
}
