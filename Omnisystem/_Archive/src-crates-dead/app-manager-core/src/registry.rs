//! Application registry with O(1) lookup

use dashmap::DashMap;
use std::sync::Arc;
use crate::app::{RegisteredApp, AppId};
use crate::module::{RegisteredModule, ModuleId};
use crate::error::{AppManagerResult, AppManagerError};

/// Lock-free registry for applications (O(1) lookup)
pub struct AppRegistry {
    apps: Arc<DashMap<AppId, RegisteredApp>>,
    index: Arc<DashMap<String, AppId>>,
}

impl AppRegistry {
    pub fn new() -> Self {
        Self {
            apps: Arc::new(DashMap::new()),
            index: Arc::new(DashMap::new()),
        }
    }

    /// Register an application (O(1))
    pub fn register(&self, app: RegisteredApp) -> AppManagerResult<()> {
        let app_id = app.manifest.id.clone();
        let app_name = app.manifest.name.clone();

        self.index.insert(app_name.clone(), app_id.clone());
        self.apps.insert(app_id, app);

        Ok(())
    }

    /// Get application by ID (O(1))
    pub fn get_by_id(&self, app_id: &AppId) -> Option<RegisteredApp> {
        self.apps.get(app_id).map(|entry| entry.clone())
    }

    /// Get application by name (O(1) via index)
    pub fn get_by_name(&self, name: &str) -> Option<RegisteredApp> {
        self.index.get(name).and_then(|app_id| {
            self.apps.get(app_id.value()).map(|entry| entry.clone())
        })
    }

    /// Get all applications (snapshot)
    pub fn list_all(&self) -> Vec<RegisteredApp> {
        self.apps.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Get count of registered apps
    pub fn count(&self) -> usize {
        self.apps.len()
    }

    /// Unregister application by ID
    pub fn unregister(&self, app_id: &AppId) -> AppManagerResult<()> {
        if let Some((_, app)) = self.apps.remove(app_id) {
            self.index.remove(&app.manifest.name);
            Ok(())
        } else {
            Err(AppManagerError::AppNotFound(app_id.to_string()))
        }
    }

    /// Check if app exists
    pub fn exists(&self, app_id: &AppId) -> bool {
        self.apps.contains_key(app_id)
    }
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AppRegistry {
    fn clone(&self) -> Self {
        Self {
            apps: Arc::clone(&self.apps),
            index: Arc::clone(&self.index),
        }
    }
}

/// Lock-free registry for modules
pub struct ModuleRegistry {
    modules: Arc<DashMap<ModuleId, RegisteredModule>>,
    by_app: Arc<DashMap<crate::app::AppId, Vec<ModuleId>>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(DashMap::new()),
            by_app: Arc::new(DashMap::new()),
        }
    }

    /// Register a module (O(1))
    pub fn register(&self, module: RegisteredModule) -> AppManagerResult<()> {
        let module_id = module.manifest.id.clone();
        let app_id = module.manifest.app_id.clone();

        // Add to modules map
        self.modules.insert(module_id.clone(), module);

        // Update app -> modules index
        self.by_app.entry(app_id).or_default().push(module_id);

        Ok(())
    }

    /// Get module by ID (O(1))
    pub fn get(&self, module_id: &ModuleId) -> Option<RegisteredModule> {
        self.modules.get(module_id).map(|entry| entry.clone())
    }

    /// Get all modules for an app
    pub fn get_by_app(&self, app_id: &crate::app::AppId) -> Vec<RegisteredModule> {
        if let Some(module_ids) = self.by_app.get(app_id) {
            module_ids
                .iter()
                .filter_map(|id| self.modules.get(id).map(|entry| entry.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get count of registered modules
    pub fn count(&self) -> usize {
        self.modules.len()
    }

    /// Unregister module by ID
    pub fn unregister(&self, module_id: &ModuleId) -> AppManagerResult<()> {
        if let Some((_, module)) = self.modules.remove(module_id) {
            if let Some(mut module_ids) = self.by_app.get_mut(&module.manifest.app_id) {
                module_ids.retain(|id| id != module_id);
            }
            Ok(())
        } else {
            Err(AppManagerError::ModuleNotFound(module_id.to_string()))
        }
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ModuleRegistry {
    fn clone(&self) -> Self {
        Self {
            modules: Arc::clone(&self.modules),
            by_app: Arc::clone(&self.by_app),
        }
    }
}

/// Search index for fast app discovery (<50ms)
pub struct SearchIndex {
    by_category: Arc<DashMap<String, Vec<AppId>>>,
    by_tag: Arc<DashMap<String, Vec<AppId>>>,
    apps: Arc<DashMap<AppId, RegisteredApp>>,
}

impl SearchIndex {
    pub fn new(apps: Arc<DashMap<AppId, RegisteredApp>>) -> Self {
        Self {
            by_category: Arc::new(DashMap::new()),
            by_tag: Arc::new(DashMap::new()),
            apps,
        }
    }

    /// Index an application for search
    pub fn index(&self, app: &RegisteredApp) -> AppManagerResult<()> {
        let app_id = app.manifest.id.clone();

        for category in &app.manifest.categories {
            self.by_category
                .entry(category.clone())
                .or_default()
                .push(app_id.clone());
        }

        for tag in &app.manifest.tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .push(app_id.clone());
        }

        Ok(())
    }

    /// Search by category (<50ms)
    pub fn search_by_category(&self, category: &str) -> Vec<RegisteredApp> {
        self.by_category
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.apps.get(id).map(|entry| entry.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search by tag (<50ms)
    pub fn search_by_tag(&self, tag: &str) -> Vec<RegisteredApp> {
        self.by_tag
            .get(tag)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.apps.get(id).map(|entry| entry.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search by multiple tags (OR query)
    pub fn search_by_tags(&self, tags: &[String]) -> Vec<RegisteredApp> {
        let mut seen = std::collections::HashSet::new();
        let mut results = Vec::new();

        for tag in tags {
            if let Some(ids) = self.by_tag.get(tag) {
                for id in ids.iter() {
                    if seen.insert(id.clone()) {
                        if let Some(app) = self.apps.get(id) {
                            results.push(app.clone());
                        }
                    }
                }
            }
        }

        results
    }

    /// Remove app from indices
    pub fn deindex(&self, app_id: &AppId) {
        for mut entry in self.by_category.iter_mut() {
            entry.retain(|id| id != app_id);
        }
        for mut entry in self.by_tag.iter_mut() {
            entry.retain(|id| id != app_id);
        }
    }
}

impl Clone for SearchIndex {
    fn clone(&self) -> Self {
        Self {
            by_category: Arc::clone(&self.by_category),
            by_tag: Arc::clone(&self.by_tag),
            apps: Arc::clone(&self.apps),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppManifest, PublisherId};

    fn create_test_app(name: &str) -> RegisteredApp {
        let manifest = AppManifest::new(
            name.to_string(),
            semver::Version::new(1, 0, 0),
            PublisherId::new(),
        );
        RegisteredApp::new(manifest)
    }

    #[test]
    fn test_app_registry_register_and_get() {
        let registry = AppRegistry::new();
        let app = create_test_app("TestApp");
        let app_id = app.manifest.id.clone();

        registry.register(app.clone()).unwrap();
        assert_eq!(registry.get_by_id(&app_id).unwrap().manifest.id, app_id);
    }

    #[test]
    fn test_app_registry_get_by_name() {
        let registry = AppRegistry::new();
        let app = create_test_app("NamedApp");

        registry.register(app.clone()).unwrap();
        assert_eq!(
            registry.get_by_name("NamedApp").unwrap().manifest.name,
            "NamedApp"
        );
    }

    #[test]
    fn test_app_registry_list() {
        let registry = AppRegistry::new();
        registry.register(create_test_app("App1")).unwrap();
        registry.register(create_test_app("App2")).unwrap();

        assert_eq!(registry.count(), 2);
        assert_eq!(registry.list_all().len(), 2);
    }

    #[test]
    fn test_app_registry_unregister() {
        let registry = AppRegistry::new();
        let app = create_test_app("ToRemove");
        let app_id = app.manifest.id.clone();

        registry.register(app).unwrap();
        assert_eq!(registry.count(), 1);

        registry.unregister(&app_id).unwrap();
        assert_eq!(registry.count(), 0);
        assert!(registry.get_by_id(&app_id).is_none());
    }

    #[test]
    fn test_search_index_by_category() {
        let apps_map = Arc::new(DashMap::new());
        let index = SearchIndex::new(apps_map.clone());

        let mut app = create_test_app("TestApp");
        app.manifest.categories = vec!["Productivity".to_string()];
        apps_map.insert(app.manifest.id.clone(), app.clone());

        index.index(&app).unwrap();
        let results = index.search_by_category("Productivity");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_index_by_tags() {
        let apps_map = Arc::new(DashMap::new());
        let index = SearchIndex::new(apps_map.clone());

        let mut app = create_test_app("TestApp");
        app.manifest.tags = vec!["tag1".to_string(), "tag2".to_string()];
        apps_map.insert(app.manifest.id.clone(), app.clone());

        index.index(&app).unwrap();
        let results = index.search_by_tags(&["tag1".to_string()]);
        assert_eq!(results.len(), 1);
    }
}
