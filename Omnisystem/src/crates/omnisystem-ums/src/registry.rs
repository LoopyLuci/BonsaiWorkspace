// Module Registry - discover and track all modules

use crate::module::{ModuleId, ModuleInfo, ModuleState};
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Entry in the module registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Module information
    pub info: ModuleInfo,

    /// Current state
    pub state: ModuleState,

    /// When added to registry
    pub registered_at: chrono::DateTime<chrono::Utc>,

    /// Last state change
    pub last_state_change: chrono::DateTime<chrono::Utc>,
}

/// Module Registry - tracks all available modules
pub struct ModuleRegistry {
    entries: Arc<DashMap<ModuleId, RegistryEntry>>,
    name_to_id: Arc<DashMap<String, ModuleId>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            name_to_id: Arc::new(DashMap::new()),
        }
    }

    /// Load registry from UMD
    pub async fn load_from_umd(
        data_manager: &crate::data::DataLayerManager,
    ) -> Result<Self> {
        let registry = Self::new();

        // Load UMD metadata
        let umd_path = data_manager.umd_source().await?;
        let registry_file = umd_path.join("registry.json");

        if registry_file.exists() {
            let contents = tokio::fs::read_to_string(&registry_file).await?;
            let entries: Vec<RegistryEntry> = serde_json::from_str(&contents)?;

            for entry in entries {
                let module_id = entry.info.id;
                let module_name = entry.info.name.clone();

                registry.entries.insert(module_id, entry);
                registry.name_to_id.insert(module_name, module_id);
            }

            tracing::info!(
                "Loaded {} modules from UMD registry",
                registry.entries.len()
            );
        } else {
            tracing::warn!("UMD registry not found: {:?}", registry_file);
        }

        Ok(registry)
    }

    /// Register a module
    pub fn register(&self, info: ModuleInfo) -> Result<()> {
        if self.name_to_id.contains_key(&info.name) {
            return Err(anyhow!("Module already registered: {}", info.name));
        }

        let entry = RegistryEntry {
            state: ModuleState::Registered,
            registered_at: chrono::Utc::now(),
            last_state_change: chrono::Utc::now(),
            info: info.clone(),
        };

        self.entries.insert(info.id, entry);
        self.name_to_id.insert(info.name.clone(), info.id);

        tracing::info!("Module registered: {}", info.name);
        Ok(())
    }

    /// Get module by ID
    pub fn get(&self, module_id: ModuleId) -> Option<RegistryEntry> {
        self.entries.get(&module_id).map(|entry| entry.value().clone())
    }

    /// Get module by name
    pub fn get_by_name(&self, name: &str) -> Option<RegistryEntry> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.entries.get(id.value()).map(|e| e.value().clone()))
    }

    /// Update module state
    pub fn update_state(&self, module_id: ModuleId, state: ModuleState) -> Result<()> {
        if let Some(mut entry) = self.entries.get_mut(&module_id) {
            entry.state = state;
            entry.last_state_change = chrono::Utc::now();
            tracing::debug!("Module state updated: {:?} -> {:?}", module_id, state);
            Ok(())
        } else {
            Err(anyhow!("Module not found: {}", module_id))
        }
    }

    /// Get all modules
    pub fn all(&self) -> Vec<RegistryEntry> {
        self.entries.iter().map(|entry| entry.value().clone()).collect()
    }

    /// Get modules by state
    pub fn by_state(&self, state: ModuleState) -> Vec<RegistryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.state == state)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get modules by phase
    pub fn by_phase(&self, phase: u32) -> Vec<RegistryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.info.phase == phase)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get modules by capability
    pub fn by_capability(&self, capability: &str) -> Vec<RegistryEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.info.capabilities.contains(&capability.to_string()))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Check if module exists
    pub fn exists(&self, name: &str) -> bool {
        self.name_to_id.contains_key(name)
    }

    /// Count modules
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Count modules by state
    pub fn count_by_state(&self, state: ModuleState) -> usize {
        self.entries.iter().filter(|e| e.state == state).count()
    }

    /// Get module dependencies (transitive)
    pub fn resolve_dependencies(&self, module_name: &str) -> Result<Vec<ModuleInfo>> {
        let mut deps = Vec::new();
        let mut visited = std::collections::HashSet::new();

        self._resolve_deps_recursive(module_name, &mut visited, &mut deps)?;

        Ok(deps)
    }

    fn _resolve_deps_recursive(
        &self,
        module_name: &str,
        visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<ModuleInfo>,
    ) -> Result<()> {
        if visited.contains(module_name) {
            return Ok(()); // Avoid cycles
        }

        visited.insert(module_name.to_string());

        let entry = self
            .get_by_name(module_name)
            .ok_or_else(|| anyhow!("Module not found: {}", module_name))?;

        for dep in &entry.info.dependencies {
            self._resolve_deps_recursive(dep, visited, result)?;
            if let Some(dep_entry) = self.get_by_name(dep) {
                result.push(dep_entry.info);
            }
        }

        Ok(())
    }

    /// Save registry to file (for UMD persistence)
    pub async fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let entries: Vec<RegistryEntry> = self.entries.iter().map(|e| e.value().clone()).collect();
        let json = serde_json::to_string_pretty(&entries)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }
}

impl Clone for ModuleRegistry {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            name_to_id: self.name_to_id.clone(),
        }
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
    use crate::module::ModuleInfo;
    use std::collections::HashMap;

    fn test_info(name: &str) -> ModuleInfo {
        ModuleInfo {
            id: ModuleId::from_name(name),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: "test".to_string(),
            dependencies: vec![],
            capabilities: vec!["demo".to_string()],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            interface_version: "1.0".to_string(),
            phase: 1,
            source_path: String::new(),
            canonical_path: String::new(),
            spec_path: String::new(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_register_and_lookup() {
        let registry = ModuleRegistry::new();
        let info = test_info("alpha");
        let id = info.id;

        registry.register(info).unwrap();

        assert_eq!(registry.count(), 1);
        assert!(registry.exists("alpha"));
        assert!(registry.get(id).is_some());
        assert_eq!(registry.get_by_name("alpha").unwrap().info.name, "alpha");
    }

    #[test]
    fn test_register_duplicate_name_fails() {
        let registry = ModuleRegistry::new();
        registry.register(test_info("alpha")).unwrap();
        assert!(registry.register(test_info("alpha")).is_err());
    }

    #[test]
    fn test_update_state_and_query_by_state() {
        let registry = ModuleRegistry::new();
        let info = test_info("alpha");
        let id = info.id;
        registry.register(info).unwrap();

        assert_eq!(registry.count_by_state(ModuleState::Registered), 1);

        registry.update_state(id, ModuleState::Running).unwrap();

        assert_eq!(registry.count_by_state(ModuleState::Registered), 0);
        assert_eq!(registry.count_by_state(ModuleState::Running), 1);
        assert_eq!(registry.get(id).unwrap().state, ModuleState::Running);
    }

    #[test]
    fn test_by_phase_and_capability() {
        let registry = ModuleRegistry::new();
        registry.register(test_info("alpha")).unwrap();
        registry.register(test_info("beta")).unwrap();

        assert_eq!(registry.by_phase(1).len(), 2);
        assert_eq!(registry.by_capability("demo").len(), 2);
        assert_eq!(registry.by_capability("nonexistent").len(), 0);
    }
}
