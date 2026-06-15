use dashmap::DashMap;
use module_interfaces::{ModuleError, ModuleId, ModuleInfo, ModuleVersion};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct ModuleRegistry {
    modules: Arc<DashMap<String, ModuleInfo>>,
    index_by_name: Arc<DashMap<String, String>>,
    index_by_tag: Arc<DashMap<String, Vec<String>>>,
    index_by_capability: Arc<DashMap<String, Vec<String>>>,
    version_index: Arc<DashMap<String, Vec<ModuleVersion>>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        info!("Creating new ModuleRegistry (lock-free, DashMap-based)");
        Self {
            modules: Arc::new(DashMap::new()),
            index_by_name: Arc::new(DashMap::new()),
            index_by_tag: Arc::new(DashMap::new()),
            index_by_capability: Arc::new(DashMap::new()),
            version_index: Arc::new(DashMap::new()),
        }
    }

    pub fn register_module(&self, module: ModuleInfo) -> Result<(), ModuleError> {
        let module_id = module.id.as_str().to_string();
        debug!("Registering module: {} v{}", module_id, module.version.to_string_canonical());

        if self.modules.contains_key(&module_id) {
            warn!("Module already registered: {}", module_id);
            return Err(ModuleError::AlreadyLoaded(module_id));
        }

        self.modules.insert(module_id.clone(), module.clone());

        self.index_by_name.insert(module.name.to_lowercase(), module_id.clone());

        for tag in &module.tags {
            self.index_by_tag
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(module_id.clone());
        }

        for capability in &module.capabilities {
            self.index_by_capability
                .entry(capability.name.clone())
                .or_insert_with(Vec::new)
                .push(module_id.clone());
        }

        self.version_index
            .entry(module_id.clone())
            .or_insert_with(Vec::new)
            .push(module.version.clone());

        info!("Module registered successfully: {}", module_id);
        Ok(())
    }

    pub fn unregister_module(&self, module_id: &str) -> Result<(), ModuleError> {
        debug!("Unregistering module: {}", module_id);

        let module = self
            .modules
            .remove(module_id)
            .ok_or_else(|| ModuleError::NotFound(module_id.to_string()))?
            .1;

        self.index_by_name.remove(&module.name.to_lowercase());

        for tag in &module.tags {
            if let Some(mut ids) = self.index_by_tag.get_mut(tag) {
                ids.retain(|id| id != module_id);
            }
        }

        for capability in &module.capabilities {
            if let Some(mut ids) = self.index_by_capability.get_mut(&capability.name) {
                ids.retain(|id| id != module_id);
            }
        }

        info!("Module unregistered: {}", module_id);
        Ok(())
    }

    pub fn get_module(&self, module_id: &str) -> Result<ModuleInfo, ModuleError> {
        debug!("Retrieving module: {}", module_id);
        self.modules
            .get(module_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| ModuleError::NotFound(module_id.to_string()))
    }

    pub fn find_by_name(&self, name: &str) -> Result<ModuleInfo, ModuleError> {
        debug!("Finding module by name: {}", name);
        let name_lower = name.to_lowercase();
        self.index_by_name
            .get(&name_lower)
            .and_then(|entry| {
                let module_id = entry.value().clone();
                self.modules.get(&module_id).map(|m| m.clone())
            })
            .ok_or_else(|| ModuleError::NotFound(format!("Module not found: {}", name)))
    }

    pub fn find_by_tag(&self, tag: &str) -> Result<Vec<ModuleInfo>, ModuleError> {
        debug!("Finding modules by tag: {}", tag);
        self.index_by_tag
            .get(tag)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter_map(|id| self.modules.get(id).map(|m| m.clone()))
                    .collect()
            })
            .unwrap_or_default()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .or(Ok(Vec::new()))
    }

    pub fn find_by_capability(&self, capability: &str) -> Result<Vec<ModuleInfo>, ModuleError> {
        debug!("Finding modules by capability: {}", capability);
        self.index_by_capability
            .get(capability)
            .map(|entry| {
                entry
                    .value()
                    .iter()
                    .filter_map(|id| self.modules.get(id).map(|m| m.clone()))
                    .collect()
            })
            .unwrap_or_default()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .or(Ok(Vec::new()))
    }

    pub fn list_all_modules(&self) -> Vec<ModuleInfo> {
        debug!("Listing all modules");
        self.modules.iter().map(|entry| entry.value().clone()).collect()
    }

    pub fn count_modules(&self) -> usize {
        self.modules.len()
    }

    pub fn exists(&self, module_id: &str) -> bool {
        self.modules.contains_key(module_id)
    }

    pub fn update_module(&self, module: ModuleInfo) -> Result<(), ModuleError> {
        let module_id = module.id.as_str().to_string();
        debug!("Updating module: {}", module_id);

        if !self.modules.contains_key(&module_id) {
            return Err(ModuleError::NotFound(module_id));
        }

        self.modules.insert(module_id.clone(), module);
        info!("Module updated: {}", module_id);
        Ok(())
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
            index_by_name: Arc::clone(&self.index_by_name),
            index_by_tag: Arc::clone(&self.index_by_tag),
            index_by_capability: Arc::clone(&self.index_by_capability),
            version_index: Arc::clone(&self.version_index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use module_interfaces::{ModuleCapability, ModuleDep, ModuleVersion, VersionConstraint, VersionConstraintType};

    fn create_test_module(id: &str, name: &str) -> ModuleInfo {
        ModuleInfo {
            id: ModuleId::new(id),
            name: name.to_string(),
            version: ModuleVersion {
                major: 1,
                minor: 0,
                patch: 0,
                pre_release: None,
                build: None,
            },
            description: "Test module".to_string(),
            author: Some("Test Author".to_string()),
            license: Some("Apache-2.0".to_string()),
            repository: None,
            documentation: None,
            capabilities: vec![ModuleCapability {
                name: "test".to_string(),
                description: None,
                version: None,
            }],
            dependencies: vec![],
            tags: vec!["test".to_string()],
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ModuleRegistry::new();
        assert_eq!(registry.count_modules(), 0);
    }

    #[test]
    fn test_register_module() {
        let registry = ModuleRegistry::new();
        let module = create_test_module("test-module", "Test Module");

        assert!(registry.register_module(module).is_ok());
        assert_eq!(registry.count_modules(), 1);
    }

    #[test]
    fn test_get_module() {
        let registry = ModuleRegistry::new();
        let module = create_test_module("test-module", "Test Module");

        registry.register_module(module.clone()).unwrap();
        let retrieved = registry.get_module("test-module").unwrap();
        assert_eq!(retrieved.id.as_str(), "test-module");
    }

    #[test]
    fn test_find_by_name() {
        let registry = ModuleRegistry::new();
        let module = create_test_module("test-module", "Test Module");

        registry.register_module(module).unwrap();
        let found = registry.find_by_name("Test Module").unwrap();
        assert_eq!(found.id.as_str(), "test-module");
    }

    #[test]
    fn test_find_by_tag() {
        let registry = ModuleRegistry::new();
        let module = create_test_module("test-module", "Test Module");

        registry.register_module(module).unwrap();
        let found = registry.find_by_tag("test").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id.as_str(), "test-module");
    }

    #[test]
    fn test_find_by_capability() {
        let registry = ModuleRegistry::new();
        let module = create_test_module("test-module", "Test Module");

        registry.register_module(module).unwrap();
        let found = registry.find_by_capability("test").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id.as_str(), "test-module");
    }

    #[test]
    fn test_unregister_module() {
        let registry = ModuleRegistry::new();
        let module = create_test_module("test-module", "Test Module");

        registry.register_module(module).unwrap();
        assert_eq!(registry.count_modules(), 1);

        registry.unregister_module("test-module").unwrap();
        assert_eq!(registry.count_modules(), 0);
    }

    #[test]
    fn test_list_all_modules() {
        let registry = ModuleRegistry::new();
        let module1 = create_test_module("module-1", "Module 1");
        let module2 = create_test_module("module-2", "Module 2");

        registry.register_module(module1).unwrap();
        registry.register_module(module2).unwrap();

        let modules = registry.list_all_modules();
        assert_eq!(modules.len(), 2);
    }

    #[test]
    fn test_exists() {
        let registry = ModuleRegistry::new();
        let module = create_test_module("test-module", "Test Module");

        assert!(!registry.exists("test-module"));
        registry.register_module(module).unwrap();
        assert!(registry.exists("test-module"));
    }

    #[test]
    fn test_update_module() {
        let registry = ModuleRegistry::new();
        let mut module = create_test_module("test-module", "Test Module");

        registry.register_module(module.clone()).unwrap();

        module.version.minor = 1;
        assert!(registry.update_module(module).is_ok());

        let updated = registry.get_module("test-module").unwrap();
        assert_eq!(updated.version.minor, 1);
    }
}
