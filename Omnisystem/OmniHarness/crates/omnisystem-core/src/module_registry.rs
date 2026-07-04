//! Module Registry - Central management of all modules
//!
//! The registry is responsible for:
//! - Registering/unregistering modules
//! - Tracking module state
//! - Managing dependencies
//! - Providing module discovery

use dashmap::DashMap;
use std::sync::Arc;
use crate::module_system::{OmniModule, ModuleMetadata};
use crate::error::{Error, Result};

/// Global module registry
pub struct ModuleRegistry {
    modules: Arc<DashMap<String, ModuleInfo>>,
}

struct ModuleInfo {
    metadata: ModuleMetadata,
    dependencies: Vec<String>,
}

impl ModuleRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            modules: Arc::new(DashMap::new()),
        }
    }

    /// Register a module
    pub fn register(&self, module: &dyn OmniModule) -> Result<()> {
        let name = module.name();

        if self.modules.contains_key(name) {
            return Err(Error::ModuleAlreadyExists(name.to_string()));
        }

        let info = ModuleInfo {
            metadata: module.metadata(),
            dependencies: module.dependencies(),
        };

        self.modules.insert(name.to_string(), info);
        Ok(())
    }

    /// Unregister a module
    pub fn unregister(&self, name: &str) -> Result<()> {
        self.modules
            .remove(name)
            .ok_or_else(|| Error::ModuleNotFound(name.to_string()))?;
        Ok(())
    }

    /// Check if module exists
    pub fn exists(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// Get module count
    pub fn count(&self) -> usize {
        self.modules.len()
    }

    /// Get all module names
    pub fn list_modules(&self) -> Vec<String> {
        self.modules
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get module metadata
    pub fn get_metadata(&self, name: &str) -> Result<ModuleMetadata> {
        self.modules
            .get(name)
            .map(|entry| entry.metadata.clone())
            .ok_or_else(|| Error::ModuleNotFound(name.to_string()))
    }

    /// Get module dependencies
    pub fn get_dependencies(&self, name: &str) -> Result<Vec<String>> {
        self.modules
            .get(name)
            .map(|entry| entry.dependencies.clone())
            .ok_or_else(|| Error::ModuleNotFound(name.to_string()))
    }

    /// Resolve dependency order (topological sort)
    pub fn resolve_dependencies(&self, modules: &[&str]) -> Result<Vec<String>> {
        let mut resolved = Vec::new();
        let mut visiting = std::collections::HashSet::new();
        let mut visited = std::collections::HashSet::new();

        for module in modules {
            self.visit(*module, &mut resolved, &mut visiting, &mut visited)?;
        }

        Ok(resolved)
    }

    fn visit(
        &self,
        name: &str,
        resolved: &mut Vec<String>,
        visiting: &mut std::collections::HashSet<String>,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Ok(());
        }

        if visiting.contains(name) {
            return Err(Error::DependencyError(format!(
                "Circular dependency detected: {}",
                name
            )));
        }

        visiting.insert(name.to_string());

        if let Ok(deps) = self.get_dependencies(name) {
            for dep in deps {
                if !dep.is_empty() {
                    self.visit(&dep, resolved, visiting, visited)?;
                }
            }
        }

        visiting.remove(name);
        visited.insert(name.to_string());
        resolved.push(name.to_string());

        Ok(())
    }

    /// Find modules by capability
    pub fn find_by_capability(&self, _capability: &str) -> Vec<String> {
        vec![] // Populated by capability system
    }

    /// Get all modules matching a filter
    pub fn filter<F>(&self, predicate: F) -> Vec<String>
    where
        F: Fn(&ModuleMetadata) -> bool,
    {
        self.modules
            .iter()
            .filter(|entry| predicate(&entry.metadata))
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_system::NoOpModule;

    #[test]
    fn test_registry_register() {
        let registry = ModuleRegistry::new();
        let module = NoOpModule::new("test", "1.0.0");

        registry.register(&module).unwrap();
        assert!(registry.exists("test"));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_registry_duplicate() {
        let registry = ModuleRegistry::new();
        let module = NoOpModule::new("test", "1.0.0");

        registry.register(&module).unwrap();
        let result = registry.register(&module);

        assert!(matches!(result, Err(Error::ModuleAlreadyExists(_))));
    }

    #[test]
    fn test_registry_list_modules() {
        let registry = ModuleRegistry::new();
        registry.register(&NoOpModule::new("mod1", "1.0.0")).unwrap();
        registry.register(&NoOpModule::new("mod2", "1.0.0")).unwrap();

        let modules = registry.list_modules();
        assert_eq!(modules.len(), 2);
    }

    #[test]
    fn test_registry_unregister() {
        let registry = ModuleRegistry::new();
        let module = NoOpModule::new("test", "1.0.0");

        registry.register(&module).unwrap();
        registry.unregister("test").unwrap();
        assert!(!registry.exists("test"));
    }

    #[test]
    fn test_registry_get_metadata() {
        let registry = ModuleRegistry::new();
        let module = NoOpModule::new("test", "1.0.0");

        registry.register(&module).unwrap();
        let metadata = registry.get_metadata("test").unwrap();

        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_registry_not_found() {
        let registry = ModuleRegistry::new();
        let result = registry.get_metadata("nonexistent");

        assert!(matches!(result, Err(Error::ModuleNotFound(_))));
    }
}
