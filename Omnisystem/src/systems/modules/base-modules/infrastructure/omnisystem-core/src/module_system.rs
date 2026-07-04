//! Universal Module System - Core trait and types
//!
//! Every feature in Omnisystem is a module implementing OmniModule.
//! Modules can be:
//! - Dynamically loaded/unloaded
//! - Enabled/disabled at runtime
//! - Composed with other modules
//! - Extended and swapped

use serde::{Deserialize, Serialize};
use crate::error::Result;

/// Universal module trait - every feature implements this
pub trait OmniModule: Send + Sync {
    /// Module name (must be unique)
    fn name(&self) -> &str;

    /// Semantic version (e.g., "1.0.0")
    fn version(&self) -> &str;

    /// Module metadata
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            name: self.name().to_string(),
            version: self.version().to_string(),
            description: "".to_string(),
            author: "".to_string(),
            repository: "".to_string(),
        }
    }

    /// Initialize module (called when enabled)
    fn initialize(&mut self) -> Result<()>;

    /// Shutdown module (called when disabled)
    fn shutdown(&mut self) -> Result<()>;

    /// Get module state
    fn state(&self) -> ModuleState;

    /// Get capabilities this module provides
    fn capabilities(&self) -> Vec<String>;

    /// Get dependencies on other modules
    fn dependencies(&self) -> Vec<String> {
        vec![]
    }

    /// Configuration schema (JSON schema)
    fn config_schema(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    /// Get current configuration
    fn config(&self) -> serde_json::Value {
        serde_json::json!({})
    }

    /// Update configuration
    fn set_config(&mut self, config: serde_json::Value) -> Result<()>;

    /// Health check - return Ok if healthy
    fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }

    /// Get module statistics
    fn stats(&self) -> ModuleStats {
        ModuleStats::default()
    }
}

/// Module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub repository: String,
}

/// Module lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleState {
    /// Not loaded
    Unloaded,
    /// Loaded but not initialized
    Loaded,
    /// Running and available
    Active,
    /// Being shut down
    Stopping,
    /// Error state
    Error,
    /// Disabled by user
    Disabled,
}

impl ModuleState {
    pub fn is_active(&self) -> bool {
        *self == ModuleState::Active
    }

    pub fn is_available(&self) -> bool {
        matches!(self, ModuleState::Active | ModuleState::Loaded)
    }
}

impl std::fmt::Display for ModuleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleState::Unloaded => write!(f, "Unloaded"),
            ModuleState::Loaded => write!(f, "Loaded"),
            ModuleState::Active => write!(f, "Active"),
            ModuleState::Stopping => write!(f, "Stopping"),
            ModuleState::Error => write!(f, "Error"),
            ModuleState::Disabled => write!(f, "Disabled"),
        }
    }
}

/// Module health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),  // Working but with issues
    Unhealthy(String), // Not working
}

/// Module statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleStats {
    pub uptime_ms: u128,
    pub error_count: usize,
    pub request_count: usize,
    pub last_error: Option<String>,
    pub custom_stats: serde_json::Value,
}

/// Simple no-op module for testing
pub struct NoOpModule {
    name: String,
    version: String,
    state: ModuleState,
    capabilities: Vec<String>,
}

impl NoOpModule {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            state: ModuleState::Unloaded,
            capabilities: vec![],
        }
    }

    pub fn with_capabilities(mut self, caps: Vec<&str>) -> Self {
        self.capabilities = caps.iter().map(|s| s.to_string()).collect();
        self
    }
}

impl OmniModule for NoOpModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn initialize(&mut self) -> Result<()> {
        self.state = ModuleState::Active;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.state = ModuleState::Unloaded;
        Ok(())
    }

    fn state(&self) -> ModuleState {
        self.state
    }

    fn capabilities(&self) -> Vec<String> {
        self.capabilities.clone()
    }

    fn set_config(&mut self, _config: serde_json::Value) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_state_active() {
        let state = ModuleState::Active;
        assert!(state.is_active());
        assert!(state.is_available());
    }

    #[test]
    fn test_module_state_disabled() {
        let state = ModuleState::Disabled;
        assert!(!state.is_active());
        assert!(!state.is_available());
    }

    #[test]
    fn test_noop_module() {
        let mut module = NoOpModule::new("test", "1.0.0");
        assert_eq!(module.name(), "test");
        assert_eq!(module.version(), "1.0.0");

        module.initialize().unwrap();
        assert_eq!(module.state(), ModuleState::Active);

        module.shutdown().unwrap();
        assert_eq!(module.state(), ModuleState::Unloaded);
    }
}
