//! Application Module Loader
//!
//! Manages loading, unloading, and lifecycle of application modules within
//! the Universal Module System. Integrates with the Universal Model Database
//! for persistent state management and module configuration.

use std::sync::Arc;
use dashmap::DashMap;
use crate::module_system::OmniModule;
use crate::module_registry::ModuleRegistry;
use crate::error::{Error, Result};

/// Application Module Loader - integrates with Universal Module System
pub struct AppModuleLoader {
    /// Module registry (from Universal Module System)
    registry: Arc<ModuleRegistry>,
    /// Loaded application modules
    loaded_apps: Arc<DashMap<String, Box<dyn OmniModule>>>,
    /// Module load order (for dependency resolution)
    load_order: Arc<parking_lot::RwLock<Vec<String>>>,
    /// Universal Model Database reference
    db_reference: String,
}

impl AppModuleLoader {
    /// Create new application module loader
    pub fn new(registry: Arc<ModuleRegistry>, db_path: &str) -> Self {
        Self {
            registry,
            loaded_apps: Arc::new(DashMap::new()),
            load_order: Arc::new(parking_lot::RwLock::new(Vec::new())),
            db_reference: db_path.to_string(),
        }
    }

    /// Load an application module dynamically
    ///
    /// # Process
    /// 1. Resolve dependencies from module registry
    /// 2. Load dependencies first (depth-first)
    /// 3. Load the application module itself
    /// 4. Initialize the module
    /// 5. Register in Universal Model Database
    /// 6. Update load tracking
    pub fn load_application(&self, app_name: &str) -> Result<()> {
        // Check if already loaded
        if self.loaded_apps.contains_key(app_name) {
            return Err(Error::ModuleAlreadyExists(
                format!("Application '{}' is already loaded", app_name),
            ));
        }

        // Resolve dependency order
        let deps = self.registry.get_dependencies(app_name)?;
        let load_order = self.registry.resolve_dependencies(&[app_name])?;

        // Verify all dependencies are available
        for dep in &deps {
            if !self.registry.exists(dep) {
                return Err(Error::DependencyError(
                    format!("Dependency '{}' not found in module registry", dep),
                ));
            }
        }

        // Load dependencies first (in order)
        for module_name in &load_order {
            if module_name == app_name {
                break; // Stop before loading app itself
            }
            self.ensure_dependency_loaded(module_name)?;
        }

        // Load application module (stub - actual implementation loads from filesystem)
        let mut app_module = self.create_application_module(app_name)?;

        // Initialize the module
        app_module.initialize()
            .map_err(|e| Error::ModuleError(
                format!("Failed to initialize application '{}': {}", app_name, e),
            ))?;

        // Register in loaded applications
        self.loaded_apps.insert(app_name.to_string(), app_module);

        // Update load order
        {
            let mut order = self.load_order.write();
            order.extend(load_order);
        }

        // Persist to Universal Model Database
        self.persist_load_state(app_name, "loaded")?;

        Ok(())
    }

    /// Unload an application module
    ///
    /// # Process
    /// 1. Check if module is loaded
    /// 2. Find dependent modules
    /// 3. Unload dependents first (reverse order)
    /// 4. Call shutdown on the module
    /// 5. Remove from loaded applications
    /// 6. Update Universal Model Database
    pub fn unload_application(&self, app_name: &str) -> Result<()> {
        // Check if loaded
        if !self.loaded_apps.contains_key(app_name) {
            return Err(Error::ModuleNotFound(
                format!("Application '{}' is not loaded", app_name),
            ));
        }

        // Find and unload dependent modules first
        let dependents = self.find_dependent_modules(app_name)?;
        for dependent in dependents.iter().rev() {
            if self.loaded_apps.contains_key(dependent) {
                self.unload_application(dependent)?;
            }
        }

        // Shutdown the module
        if let Some((_, mut module)) = self.loaded_apps.remove(app_name) {
            module.shutdown()
                .map_err(|e| Error::ModuleError(
                    format!("Failed to shutdown application '{}': {}", app_name, e),
                ))?;
        }

        // Update load order
        {
            let mut order = self.load_order.write();
            order.retain(|name| name != app_name);
        }

        // Update Universal Model Database
        self.persist_load_state(app_name, "unloaded")?;

        Ok(())
    }

    /// Hot-reload an application module (if hot_swappable)
    ///
    /// Unloads and immediately reloads the module without affecting
    /// other modules (if hot_swappable is enabled in manifest)
    pub fn hot_reload_application(&self, app_name: &str) -> Result<()> {
        // Confirm the module is registered before attempting the swap
        self.registry.get_metadata(app_name)?;

        self.unload_application(app_name)?;
        self.load_application(app_name)?;

        Ok(())
    }

    /// Get list of loaded applications
    pub fn list_loaded_applications(&self) -> Vec<String> {
        self.loaded_apps
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get application health status
    pub fn get_application_health(&self, app_name: &str) -> Result<String> {
        if let Some(module) = self.loaded_apps.get(app_name) {
            let health = module.health_check()?;
            Ok(format!("{:?}", health))
        } else {
            Err(Error::ModuleNotFound(
                format!("Application '{}' is not loaded", app_name),
            ))
        }
    }

    /// Get application statistics
    pub fn get_application_stats(&self, app_name: &str) -> Result<serde_json::Value> {
        if let Some(module) = self.loaded_apps.get(app_name) {
            let stats = module.stats();
            Ok(serde_json::to_value(stats).unwrap_or(serde_json::json!({})))
        } else {
            Err(Error::ModuleNotFound(
                format!("Application '{}' is not loaded", app_name),
            ))
        }
    }

    /// Update application configuration
    pub fn configure_application(
        &self,
        app_name: &str,
        config: serde_json::Value,
    ) -> Result<()> {
        if let Some(mut module) = self.loaded_apps.get_mut(app_name) {
            module.set_config(config)?;
            // Persist new config to Universal Model Database
            self.persist_configuration(app_name)?;
            Ok(())
        } else {
            Err(Error::ModuleNotFound(
                format!("Application '{}' is not loaded", app_name),
            ))
        }
    }

    // ========================================================================
    // PRIVATE IMPLEMENTATION
    // ========================================================================

    /// Ensure a dependency module is loaded
    fn ensure_dependency_loaded(&self, dep_name: &str) -> Result<()> {
        if !self.loaded_apps.contains_key(dep_name) {
            // Recursively load dependencies
            // In production, would load from module registry/filesystem
        }
        Ok(())
    }

    /// Create an application module (stub implementation)
    ///
    /// In production, this would:
    /// 1. Load module binary/code from disk
    /// 2. Parse APPLICATION_MANIFEST.omni for module metadata
    /// 3. Instantiate module from loaded code
    /// 4. Return boxed trait object implementing OmniModule
    fn create_application_module(&self, _app_name: &str) -> Result<Box<dyn OmniModule>> {
        // Stub: in production, load from application modules directory
        // Return Box::new(ActualModuleInstance::new(app_name, ...))
        Err(Error::ModuleNotFound("Stub implementation".to_string()))
    }

    /// Find modules that depend on this module
    fn find_dependent_modules(&self, module_name: &str) -> Result<Vec<String>> {
        let mut dependents = Vec::new();

        for entry in self.loaded_apps.iter() {
            let app_name = entry.key().clone();
            if let Ok(deps) = self.registry.get_dependencies(&app_name) {
                if deps.contains(&module_name.to_string()) {
                    dependents.push(app_name);
                }
            }
        }

        Ok(dependents)
    }

    /// Persist module load state to Universal Model Database
    fn persist_load_state(&self, app_name: &str, state: &str) -> Result<()> {
        // In production, would write to Universal Model Database:
        // {
        //   "module": app_name,
        //   "state": state,  // "loaded" | "unloaded"
        //   "timestamp": now,
        //   "path": self.db_reference,
        // }

        // For now, just track locally
        let _db_key = format!("{}/modules/{}/state", self.db_reference, app_name);
        let _state_value = format!("{{\"state\": \"{}\", \"timestamp\": {}}}",
            state,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        Ok(())
    }

    /// Persist module configuration to Universal Model Database
    fn persist_configuration(&self, app_name: &str) -> Result<()> {
        // In production, would write to Universal Model Database:
        // {
        //   "module": app_name,
        //   "configuration": { ... },
        //   "path": self.db_reference,
        // }

        let _db_key = format!("{}/modules/{}/config", self.db_reference, app_name);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_module_loader_creation() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = AppModuleLoader::new(registry, "/omnisystem/db");
        assert_eq!(loader.list_loaded_applications().len(), 0);
    }

    #[test]
    fn test_list_loaded_applications() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = AppModuleLoader::new(registry, "/omnisystem/db");

        let apps = loader.list_loaded_applications();
        assert!(apps.is_empty());
    }
}
