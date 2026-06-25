//! Omnisystem Bonsai Ecosystem Module - Desktop launcher, runtime, and orchestration

use omnisystem_core::{OmniModule, ModuleState, ModuleMetadata, Result, Error};
use omnisystem_core::module_system::HealthStatus;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonsaiConfig {
    pub launcher_enabled: bool,
    pub runtime_enabled: bool,
    pub orchestration_enabled: bool,
    pub ui_theme: String,
    pub auto_start: bool,
}

impl Default for BonsaiConfig {
    fn default() -> Self {
        Self {
            launcher_enabled: true,
            runtime_enabled: true,
            orchestration_enabled: true,
            ui_theme: "auto".to_string(),
            auto_start: true,
        }
    }
}

pub struct BonsaiEcosystemModule {
    name: String,
    version: String,
    state: ModuleState,
    config: Arc<Mutex<BonsaiConfig>>,
    capabilities: Vec<String>,
}

impl BonsaiEcosystemModule {
    pub fn new(config: BonsaiConfig) -> Result<Self> {
        Ok(Self {
            name: "omnisystem-bonsai-ecosystem".to_string(),
            version: "1.0.0".to_string(),
            state: ModuleState::Unloaded,
            config: Arc::new(Mutex::new(config)),
            capabilities: vec![
                "bonsai:launcher".to_string(),
                "bonsai:runtime".to_string(),
                "bonsai:orchestration".to_string(),
                "bonsai:ui".to_string(),
            ],
        })
    }
}

impl OmniModule for BonsaiEcosystemModule {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            description: "Bonsai Ecosystem - Desktop launcher, UOSC runtime, and orchestration".to_string(),
            author: "Omnisystem Team".to_string(),
            repository: "https://github.com/omnisystem/omnisystem".to_string(),
        }
    }

    fn initialize(&mut self) -> omnisystem_core::Result<()> {
        self.state = ModuleState::Active;
        log::info!("Bonsai Ecosystem module initialized");
        Ok(())
    }

    fn shutdown(&mut self) -> omnisystem_core::Result<()> {
        self.state = ModuleState::Unloaded;
        log::info!("Bonsai Ecosystem module shutdown");
        Ok(())
    }

    fn state(&self) -> ModuleState { self.state }

    fn capabilities(&self) -> Vec<String> { self.capabilities.clone() }

    fn dependencies(&self) -> Vec<String> { vec![] }

    fn set_config(&mut self, config: serde_json::Value) -> omnisystem_core::Result<()> {
        let new_config: BonsaiConfig = serde_json::from_value(config)
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
    fn test_bonsai_module_creation() {
        let config = BonsaiConfig::default();
        let module = BonsaiEcosystemModule::new(config).unwrap();
        assert_eq!(module.name(), "omnisystem-bonsai-ecosystem");
    }

    #[test]
    fn test_bonsai_module_initialize() {
        let config = BonsaiConfig::default();
        let mut module = BonsaiEcosystemModule::new(config).unwrap();
        module.initialize().unwrap();
        assert_eq!(module.state(), ModuleState::Active);
    }
}
