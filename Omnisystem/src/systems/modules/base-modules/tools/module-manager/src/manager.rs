//! Core Module Manager - Central orchestration for module lifecycle

use crate::{
    adapters::LanguageAdapterTrait, LoadedModule, ModuleId, ModuleManagerError,
    PackageRegistry, Result,
};
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Universal Module Manager
pub struct ModuleManager {
    /// Loaded modules cache
    loaded_modules: Arc<DashMap<String, LoadedModule>>,
    /// Language adapters
    adapters: Arc<DashMap<String, Arc<dyn LanguageAdapterTrait>>>,
    /// Package registry
    registry: Arc<dyn PackageRegistry>,
    /// Module cache directory
    cache_dir: PathBuf,
}

impl ModuleManager {
    /// Create new module manager
    pub fn new(registry: Arc<dyn PackageRegistry>, cache_dir: PathBuf) -> Self {
        Self {
            loaded_modules: Arc::new(DashMap::new()),
            adapters: Arc::new(DashMap::new()),
            registry,
            cache_dir,
        }
    }

    /// Register language adapter
    pub fn register_adapter(
        &self,
        language: &str,
        adapter: Arc<dyn LanguageAdapterTrait>,
    ) -> Result<()> {
        self.adapters.insert(language.to_string(), adapter);
        log::info!("Registered adapter for language: {}", language);
        Ok(())
    }

    /// Load module by ID (fast path with caching)
    pub fn load_module(&self, id: &ModuleId) -> Result<LoadedModule> {
        let full_id = id.full_id();

        // Check cache first (O(1) lookup)
        if let Some(module) = self.loaded_modules.get(&full_id) {
            return Ok(module.clone());
        }

        // Get language adapter
        let adapter = self
            .adapters
            .get(&id.language)
            .ok_or_else(|| {
                ModuleManagerError::LanguageAdapterNotFound(id.language.clone())
            })?;

        // Load metadata
        let metadata = self.registry.get_metadata(id)?;

        // Construct module location
        let location = self.cache_dir.join(&id.namespace).join(&id.name);

        // Download/extract if needed
        if !location.exists() {
            self.registry.download_module(id, &location)?;
            adapter.extract(&location)?;
        }

        // Verify checksum (robust)
        adapter.verify_checksum(&location, &metadata.checksum)?;

        let loaded = LoadedModule {
            metadata,
            location,
            loaded_at: chrono::Utc::now(),
        };

        // Cache result
        self.loaded_modules
            .insert(full_id, loaded.clone());

        Ok(loaded)
    }

    /// Unload module (cleanup)
    pub fn unload_module(&self, id: &ModuleId) -> Result<()> {
        let full_id = id.full_id();

        if let Some((_, module)) = self.loaded_modules.remove(&full_id) {
            let adapter = self
                .adapters
                .get(&id.language)
                .ok_or_else(|| {
                    ModuleManagerError::LanguageAdapterNotFound(id.language.clone())
                })?;

            adapter.cleanup(&module.location)?;
            log::info!("Unloaded module: {}", full_id);
        }

        Ok(())
    }

    /// Get loaded modules
    pub fn loaded_modules(&self) -> HashMap<String, LoadedModule> {
        self.loaded_modules
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Clear cache
    pub fn clear_cache(&self) -> Result<()> {
        self.loaded_modules.clear();
        std::fs::remove_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        log::info!("Module cache cleared");
        Ok(())
    }

    /// Get module statistics
    pub fn stats(&self) -> ModuleManagerStats {
        ModuleManagerStats {
            loaded_count: self.loaded_modules.len(),
            adapters_count: self.adapters.len(),
            cache_size_mb: self.get_cache_size().unwrap_or(0) as f64 / 1024.0 / 1024.0,
        }
    }

    /// Get cache size in bytes
    fn get_cache_size(&self) -> Result<u64> {
        let mut total = 0u64;
        for entry in walkdir::WalkDir::new(&self.cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                total += metadata.len();
            }
        }
        Ok(total)
    }
}

/// Module manager statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleManagerStats {
    pub loaded_count: usize,
    pub adapters_count: usize,
    pub cache_size_mb: f64,
}

impl Clone for LoadedModule {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            location: self.location.clone(),
            loaded_at: self.loaded_at,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_manager_creation() {
        // Would need mock registry
        // let manager = ModuleManager::new(...);
        // assert_eq!(manager.loaded_modules().len(), 0);
    }
}
