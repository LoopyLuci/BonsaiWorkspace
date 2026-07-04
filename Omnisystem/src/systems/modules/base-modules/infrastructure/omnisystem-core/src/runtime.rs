//! Omnisystem Runtime - Core orchestration engine
//!
//! Manages the complete lifecycle of the Omnisystem:
//! - Module loading and initialization
//! - Capability management
//! - Mode switching (OmniOS ↔ Bonsai)
//! - System health monitoring

use crate::{
    OmniMode, module_registry::ModuleRegistry, capability_system::CapabilityManager,
    data_manager::DataManager, error::Result,
};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Omnisystem Runtime - Central orchestration
pub struct OmnisystemRuntime {
    mode: Arc<RwLock<OmniMode>>,
    registry: Arc<ModuleRegistry>,
    capabilities: Arc<CapabilityManager>,
    data_manager: Arc<DataManager>,
    config: Arc<RwLock<RuntimeConfig>>,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub mode: OmniMode,
    pub enable_telemetry: bool,
    pub enable_auto_update: bool,
    pub max_concurrent_modules: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mode: OmniMode::OmniOS,
            enable_telemetry: true,
            enable_auto_update: true,
            max_concurrent_modules: 64,
        }
    }
}

impl OmnisystemRuntime {
    /// Create new runtime with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(RuntimeConfig::default())
    }

    /// Create runtime with custom configuration
    pub fn with_config(config: RuntimeConfig) -> Result<Self> {
        let data_manager = Arc::new(DataManager::new()?);

        Ok(Self {
            mode: Arc::new(RwLock::new(config.mode)),
            registry: Arc::new(ModuleRegistry::new()),
            capabilities: Arc::new(CapabilityManager::new()),
            data_manager,
            config: Arc::new(RwLock::new(config)),
        })
    }

    /// Get current mode
    pub fn mode(&self) -> OmniMode {
        *self.mode.read()
    }

    /// Switch to different mode
    pub fn set_mode(&self, mode: OmniMode) -> Result<()> {
        // Validate mode switch is safe
        match mode {
            OmniMode::OmniOS => {
                // Can always switch to full mode
                *self.mode.write() = mode;
            }
            OmniMode::Bonsai => {
                // Switching to Bonsai - ensure minimal modules are active
                // This would disable non-essential capabilities
                *self.mode.write() = mode;
            }
        }
        Ok(())
    }

    /// Get module registry
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    /// Get capability manager
    pub fn capabilities(&self) -> &CapabilityManager {
        &self.capabilities
    }

    /// Get data manager
    pub fn data_manager(&self) -> &DataManager {
        &self.data_manager
    }

    /// Get current configuration
    pub fn config(&self) -> RuntimeConfig {
        self.config.read().clone()
    }

    /// Update configuration
    pub fn set_config(&self, config: RuntimeConfig) -> Result<()> {
        *self.config.write() = config;
        Ok(())
    }

    /// Get system status
    pub fn status(&self) -> SystemStatus {
        SystemStatus {
            mode: self.mode(),
            modules_loaded: self.registry.count(),
            capabilities_enabled: self.capabilities.stats().enabled,
            capabilities_total: self.capabilities.stats().total,
        }
    }

    /// Health check for the entire runtime
    pub fn health_check(&self) -> Result<HealthStatus> {
        let modules = self.registry.count();
        let caps = self.capabilities.stats();

        if modules == 0 || caps.total == 0 {
            return Ok(HealthStatus::Degraded(
                "No modules or capabilities loaded".to_string(),
            ));
        }

        Ok(HealthStatus::Healthy)
    }

    /// Get runtime statistics
    pub fn stats(&self) -> RuntimeStats {
        let caps = self.capabilities.stats();
        let disk_usage = self.data_manager.disk_usage().ok();

        RuntimeStats {
            mode: self.mode(),
            modules_loaded: self.registry.count(),
            capabilities_total: caps.total,
            capabilities_enabled: caps.enabled,
            disk_usage_mb: disk_usage.map(|du| du.total_mb()),
        }
    }
}

impl Default for OmnisystemRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default runtime")
    }
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

/// System status snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub mode: OmniMode,
    pub modules_loaded: usize,
    pub capabilities_enabled: usize,
    pub capabilities_total: usize,
}

/// Runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStats {
    pub mode: OmniMode,
    pub modules_loaded: usize,
    pub capabilities_total: usize,
    pub capabilities_enabled: usize,
    pub disk_usage_mb: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = OmnisystemRuntime::new().unwrap();
        assert_eq!(runtime.mode(), OmniMode::OmniOS);
        assert_eq!(runtime.registry().count(), 0);
    }

    #[test]
    fn test_mode_switching() {
        let runtime = OmnisystemRuntime::new().unwrap();

        runtime.set_mode(OmniMode::Bonsai).unwrap();
        assert_eq!(runtime.mode(), OmniMode::Bonsai);

        runtime.set_mode(OmniMode::OmniOS).unwrap();
        assert_eq!(runtime.mode(), OmniMode::OmniOS);
    }

    #[test]
    fn test_runtime_status() {
        let runtime = OmnisystemRuntime::new().unwrap();
        let status = runtime.status();

        assert_eq!(status.mode, OmniMode::OmniOS);
        assert_eq!(status.modules_loaded, 0);
    }

    #[test]
    fn test_runtime_config() {
        let runtime = OmnisystemRuntime::new().unwrap();
        let config = runtime.config();

        assert_eq!(config.mode, OmniMode::OmniOS);
        assert!(config.enable_telemetry);
    }

    #[test]
    fn test_runtime_health_check() {
        let runtime = OmnisystemRuntime::new().unwrap();
        let health = runtime.health_check();

        // No modules = degraded
        assert!(matches!(health, Ok(HealthStatus::Degraded(_))));
    }
}
