//! Omnisystem Networking Module - P2P networking with multi-path routing

use omnisystem_core::{OmniModule, ModuleState, ModuleMetadata, Result, Error};
use omnisystem_core::module_system::HealthStatus;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingConfig {
    pub p2p_enabled: bool,
    pub relay_enabled: bool,
    pub multi_path_routing: bool,
    pub max_connections: usize,
    pub listen_port: u16,
}

impl Default for NetworkingConfig {
    fn default() -> Self {
        Self {
            p2p_enabled: true,
            relay_enabled: true,
            multi_path_routing: true,
            max_connections: 1024,
            listen_port: 0,  // Random port
        }
    }
}

pub struct NetworkingModule {
    name: String,
    version: String,
    state: ModuleState,
    config: Arc<Mutex<NetworkingConfig>>,
    capabilities: Vec<String>,
}

impl NetworkingModule {
    pub fn new(config: NetworkingConfig) -> Result<Self> {
        Ok(Self {
            name: "omnisystem-networking".to_string(),
            version: "1.0.0".to_string(),
            state: ModuleState::Unloaded,
            config: Arc::new(Mutex::new(config)),
            capabilities: vec![
                "networking:p2p".to_string(),
                "networking:relay".to_string(),
                "networking:multi-path".to_string(),
                "networking:encryption".to_string(),
            ],
        })
    }
}

impl OmniModule for NetworkingModule {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            description: "P2P networking with multi-path routing".to_string(),
            author: "Omnisystem Team".to_string(),
            repository: "https://github.com/omnisystem/omnisystem".to_string(),
        }
    }

    fn initialize(&mut self) -> omnisystem_core::Result<()> {
        self.state = ModuleState::Active;
        log::info!("Networking module initialized");
        Ok(())
    }

    fn shutdown(&mut self) -> omnisystem_core::Result<()> {
        self.state = ModuleState::Unloaded;
        log::info!("Networking module shutdown");
        Ok(())
    }

    fn state(&self) -> ModuleState { self.state }

    fn capabilities(&self) -> Vec<String> { self.capabilities.clone() }

    fn dependencies(&self) -> Vec<String> { vec![] }

    fn set_config(&mut self, config: serde_json::Value) -> omnisystem_core::Result<()> {
        let new_config: NetworkingConfig = serde_json::from_value(config)
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
    fn test_networking_module_creation() {
        let config = NetworkingConfig::default();
        let module = NetworkingModule::new(config).unwrap();
        assert_eq!(module.name(), "omnisystem-networking");
    }
}
