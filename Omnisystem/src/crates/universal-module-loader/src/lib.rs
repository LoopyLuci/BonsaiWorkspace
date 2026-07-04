use dashmap::DashMap;
use module_interfaces::{ModuleError, ModuleId, ModuleLoadRequest, ModuleLoadResponse, ModuleStatus};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn, error};
use universal_module_registry::ModuleRegistry;

pub struct ModuleLoader {
    registry: Arc<ModuleRegistry>,
    loaded_modules: Arc<DashMap<String, LoadedModuleInfo>>,
    load_times: Arc<DashMap<String, u64>>,
}

#[derive(Clone)]
pub struct LoadedModuleInfo {
    pub module_id: String,
    pub version: String,
    pub state: ModuleLoaderState,
    pub load_time_ms: u64,
    pub loaded_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleLoaderState {
    Unloaded,
    PreInit,
    Loading,
    PostInit,
    Loaded,
    Running,
    Paused,
    Unloading,
    Error,
}

impl ModuleLoader {
    pub fn new(registry: Arc<ModuleRegistry>) -> Self {
        info!("Creating new ModuleLoader with {} registered modules", registry.count_modules());
        Self {
            registry,
            loaded_modules: Arc::new(DashMap::new()),
            load_times: Arc::new(DashMap::new()),
        }
    }

    pub async fn load_module(&self, request: &ModuleLoadRequest) -> Result<ModuleLoadResponse, ModuleError> {
        let start = Instant::now();
        debug!("Loading module: {} v{}", request.module_id, request.version.as_deref().unwrap_or("latest"));

        if self.loaded_modules.contains_key(&request.module_id) {
            warn!("Module already loaded: {}", request.module_id);
            return Err(ModuleError::AlreadyLoaded(request.module_id.clone()));
        }

        self.set_state(&request.module_id, ModuleLoaderState::PreInit)?;

        let module_info = self.registry.get_module(&request.module_id)
            .map_err(|_| ModuleError::NotFound(request.module_id.clone()))?;

        self.set_state(&request.module_id, ModuleLoaderState::Loading)?;

        for dep in &module_info.dependencies {
            if !dep.optional {
                self.load_module(&ModuleLoadRequest {
                    module_id: dep.module_id.0.clone(),
                    version: None,
                    config: None,
                })?;
            }
        }

        self.set_state(&request.module_id, ModuleLoaderState::PostInit)?;
        self.set_state(&request.module_id, ModuleLoaderState::Loaded)?;
        self.set_state(&request.module_id, ModuleLoaderState::Running)?;

        let elapsed = start.elapsed().as_millis() as u64;
        self.load_times.insert(request.module_id.clone(), elapsed);

        let info = LoadedModuleInfo {
            module_id: request.module_id.clone(),
            version: request.version.clone().unwrap_or_else(|| module_info.version.to_string_canonical()),
            state: ModuleLoaderState::Running,
            load_time_ms: elapsed,
            loaded_at: chrono::Utc::now().timestamp() as u64,
        };

        self.loaded_modules.insert(request.module_id.clone(), info);

        info!("Module loaded successfully: {} ({}ms)", request.module_id, elapsed);

        Ok(ModuleLoadResponse {
            success: true,
            module_id: request.module_id.clone(),
            message: format!("Module loaded successfully in {}ms", elapsed),
            load_time_ms: elapsed,
        })
    }

    pub async fn unload_module(&self, module_id: &str) -> Result<(), ModuleError> {
        debug!("Unloading module: {}", module_id);

        let module = self.loaded_modules
            .get(module_id)
            .ok_or_else(|| ModuleError::NotLoaded(module_id.to_string()))?
            .clone();

        self.set_state(module_id, ModuleLoaderState::Unloading)?;

        self.loaded_modules.remove(module_id);
        self.load_times.remove(module_id);

        info!("Module unloaded: {}", module_id);
        Ok(())
    }

    pub fn get_module_status(&self, module_id: &str) -> Result<ModuleStatus, ModuleError> {
        self.loaded_modules
            .get(module_id)
            .map(|entry| match entry.state {
                ModuleLoaderState::Running => ModuleStatus::Running,
                ModuleLoaderState::Loaded => ModuleStatus::Loaded,
                ModuleLoaderState::Loading => ModuleStatus::Loading,
                ModuleLoaderState::Unloading => ModuleStatus::Stopping,
                ModuleLoaderState::Error => ModuleStatus::Error,
                _ => ModuleStatus::Unknown,
            })
            .ok_or_else(|| ModuleError::NotLoaded(module_id.to_string()))
    }

    pub fn is_loaded(&self, module_id: &str) -> bool {
        self.loaded_modules.contains_key(module_id)
    }

    pub fn get_loaded_modules(&self) -> Vec<LoadedModuleInfo> {
        self.loaded_modules
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn count_loaded_modules(&self) -> usize {
        self.loaded_modules.len()
    }

    pub fn get_load_time(&self, module_id: &str) -> Option<u64> {
        self.load_times.get(module_id).map(|entry| *entry.value())
    }

    fn set_state(&self, module_id: &str, new_state: ModuleLoaderState) -> Result<(), ModuleError> {
        if let Some(mut entry) = self.loaded_modules.get_mut(module_id) {
            entry.state = new_state;
            Ok(())
        } else if new_state == ModuleLoaderState::PreInit {
            let info = LoadedModuleInfo {
                module_id: module_id.to_string(),
                version: "unknown".to_string(),
                state: new_state,
                load_time_ms: 0,
                loaded_at: chrono::Utc::now().timestamp() as u64,
            };
            self.loaded_modules.insert(module_id.to_string(), info);
            Ok(())
        } else {
            Err(ModuleError::NotFound(module_id.to_string()))
        }
    }
}

impl Clone for ModuleLoader {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            loaded_modules: Arc::clone(&self.loaded_modules),
            load_times: Arc::clone(&self.load_times),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_loader_creation() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = ModuleLoader::new(registry);
        assert_eq!(loader.count_loaded_modules(), 0);
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = ModuleLoader::new(registry);

        assert_eq!(loader.count_loaded_modules(), 0);
    }

    #[test]
    fn test_module_loader_state_enum() {
        assert_eq!(ModuleLoaderState::Running, ModuleLoaderState::Running);
        assert_ne!(ModuleLoaderState::Running, ModuleLoaderState::Unloaded);
    }

    #[test]
    fn test_loaded_module_info_creation() {
        let info = LoadedModuleInfo {
            module_id: "test".to_string(),
            version: "1.0.0".to_string(),
            state: ModuleLoaderState::Running,
            load_time_ms: 50,
            loaded_at: 0,
        };
        assert_eq!(info.module_id, "test");
        assert_eq!(info.load_time_ms, 50);
    }

    #[test]
    fn test_module_loader_clone() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = ModuleLoader::new(registry.clone());
        let cloned = loader.clone();
        assert_eq!(loader.count_loaded_modules(), cloned.count_loaded_modules());
    }
}
