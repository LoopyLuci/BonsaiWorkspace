//! Omnisystem Messaging Module - Sovereign SMTP/IMAP/P2P email

use omnisystem_core::{OmniModule, ModuleState, ModuleMetadata, Result, Error};
use omnisystem_core::module_system::HealthStatus;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingConfig {
    pub smtp_enabled: bool,
    pub imap_enabled: bool,
    pub p2p_enabled: bool,
    pub smtp_port: u16,
    pub imap_port: u16,
    pub encryption_enabled: bool,
    pub spam_filter_enabled: bool,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        Self {
            smtp_enabled: true,
            imap_enabled: true,
            p2p_enabled: true,
            smtp_port: 25,
            imap_port: 143,
            encryption_enabled: true,
            spam_filter_enabled: true,
        }
    }
}

pub struct MessagingModule {
    name: String,
    version: String,
    state: ModuleState,
    config: Arc<Mutex<MessagingConfig>>,
    capabilities: Vec<String>,
}

impl MessagingModule {
    pub fn new(config: MessagingConfig) -> Result<Self> {
        Ok(Self {
            name: "omnisystem-messaging".to_string(),
            version: "1.0.0".to_string(),
            state: ModuleState::Unloaded,
            config: Arc::new(Mutex::new(config)),
            capabilities: vec![
                "messaging:smtp".to_string(),
                "messaging:imap".to_string(),
                "messaging:p2p".to_string(),
                "messaging:encryption".to_string(),
                "messaging:spam-filter".to_string(),
            ],
        })
    }
}

impl OmniModule for MessagingModule {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { &self.version }

    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            description: "Sovereign messaging framework with SMTP, IMAP, and P2P email".to_string(),
            author: "Omnisystem Team".to_string(),
            repository: "https://github.com/omnisystem/omnisystem".to_string(),
        }
    }

    fn initialize(&mut self) -> omnisystem_core::Result<()> {
        self.state = ModuleState::Active;
        log::info!("Messaging module initialized");
        Ok(())
    }

    fn shutdown(&mut self) -> omnisystem_core::Result<()> {
        self.state = ModuleState::Unloaded;
        log::info!("Messaging module shutdown");
        Ok(())
    }

    fn state(&self) -> ModuleState { self.state }

    fn capabilities(&self) -> Vec<String> { self.capabilities.clone() }

    fn dependencies(&self) -> Vec<String> { vec![] }

    fn set_config(&mut self, config: serde_json::Value) -> omnisystem_core::Result<()> {
        let new_config: MessagingConfig = serde_json::from_value(config)
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
    fn test_messaging_module_creation() {
        let config = MessagingConfig::default();
        let module = MessagingModule::new(config).unwrap();
        assert_eq!(module.name(), "omnisystem-messaging");
    }

    #[test]
    fn test_messaging_module_initialize() {
        let config = MessagingConfig::default();
        let mut module = MessagingModule::new(config).unwrap();
        module.initialize().unwrap();
        assert_eq!(module.state(), ModuleState::Active);
    }
}
