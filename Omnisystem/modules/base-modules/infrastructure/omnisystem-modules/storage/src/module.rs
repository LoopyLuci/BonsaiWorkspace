//! Omnisystem Storage Module - Content-addressed storage with distributed replication

use omnisystem_core::{OmniModule, ModuleState, ModuleMetadata, Result, Error};
use omnisystem_core::module_system::HealthStatus;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub cas_enabled: bool,
    pub replication_factor: usize,
    pub storage_path: String,
    pub max_object_size_mb: usize,
    pub p2p_replication: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            cas_enabled: true,
            replication_factor: 3,
            storage_path: "~/.omnisystem/modules/storage/objects".to_string(),
            max_object_size_mb: 1024,
            p2p_replication: true,
        }
    }
}

pub struct StorageModule {
    name: String,
    version: String,
    state: ModuleState,
    config: Arc<Mutex<StorageConfig>>,
    capabilities: Vec<String>,
}

impl StorageModule {
    pub fn new(config: StorageConfig) -> Result<Self> {
        Ok(Self {
            name: "omnisystem-storage".to_string(),
            version: "1.0.0".to_string(),
            state: ModuleState::Unloaded,
            config: Arc::new(Mutex::new(config)),
            capabilities: vec![
                "storage:cas".to_string(),
                "storage:replication".to_string(),
                "storage:p2p-sync".to_string(),
                "storage:compression".to_string(),
            ],
        })
    }
}

impl OmniModule for StorageModule {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            description: "Content-addressed storage with distributed replication".to_string(),
            author: "Omnisystem Team".to_string(),
            repository: "https://github.com/omnisystem/omnisystem".to_string(),
        }
    }

    fn initialize(&mut self) -> omnisystem_core::Result<()> {
        self.state = ModuleState::Active;
        log::info!("Storage module initialized");
        Ok(())
    }

    fn shutdown(&mut self) -> omnisystem_core::Result<()> {
        self.state = ModuleState::Unloaded;
        log::info!("Storage module shutdown");
        Ok(())
    }

    fn state(&self) -> ModuleState { self.state }

    fn capabilities(&self) -> Vec<String> { self.capabilities.clone() }

    fn dependencies(&self) -> Vec<String> { vec![] }

    fn set_config(&mut self, config: serde_json::Value) -> omnisystem_core::Result<()> {
        let new_config: StorageConfig = serde_json::from_value(config)
            .map_err(|e| Error::module_error(format!("Config error: {}", e)))?;
        *self.config.lock() = new_config;
        Ok(())
    }

    fn health_check(&self) -> omnisystem_core::Result<HealthStatus> {
        if self.state == ModuleState::Active {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Degraded("Not active".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_module_creation() {
        let config = StorageConfig::default();
        let module = StorageModule::new(config).unwrap();
        assert_eq!(module.name(), "omnisystem-storage");
    }
}
