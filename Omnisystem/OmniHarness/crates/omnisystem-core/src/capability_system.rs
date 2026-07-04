//! Capability System - Feature toggling and availability management
//!
//! Capabilities are individual features that can be enabled/disabled.
//! Multiple capabilities can be provided by a single module.
//! Capabilities can have dependencies on other capabilities.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::error::{Error, Result};

/// A capability is an individual feature that can be toggled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub module: String,
    pub enabled: bool,
    pub dependencies: Vec<String>,
    pub config: serde_json::Value,
}

impl Capability {
    pub fn new(name: &str, module: &str) -> Self {
        Self {
            name: name.to_string(),
            module: module.to_string(),
            enabled: false,
            dependencies: vec![],
            config: serde_json::json!({}),
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<&str>) -> Self {
        self.dependencies = deps.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }
}

/// Manages all capabilities in the system
pub struct CapabilityManager {
    capabilities: Arc<DashMap<String, Capability>>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(DashMap::new()),
        }
    }

    /// Register a capability
    pub fn register(&self, capability: Capability) -> Result<()> {
        if self.capabilities.contains_key(&capability.name) {
            return Err(Error::capability_error(format!(
                "Capability already exists: {}",
                capability.name
            )));
        }

        self.capabilities.insert(capability.name.clone(), capability);
        Ok(())
    }

    /// Unregister a capability
    pub fn unregister(&self, name: &str) -> Result<()> {
        self.capabilities
            .remove(name)
            .ok_or_else(|| Error::CapabilityNotFound(name.to_string()))?;
        Ok(())
    }

    /// Check if capability exists
    pub fn exists(&self, name: &str) -> bool {
        self.capabilities.contains_key(name)
    }

    /// Check if capability is enabled
    pub fn is_enabled(&self, name: &str) -> Result<bool> {
        self.capabilities
            .get(name)
            .map(|cap| cap.enabled)
            .ok_or_else(|| Error::CapabilityNotFound(name.to_string()))
    }

    /// Enable a capability
    pub fn enable(&self, name: &str) -> Result<()> {
        if let Some(mut cap) = self.capabilities.get_mut(name) {
            // Check dependencies first
            for dep in &cap.dependencies {
                if !self.is_enabled(dep).unwrap_or(false) {
                    return Err(Error::DependencyError(format!(
                        "Cannot enable {}: dependency {} is disabled",
                        name, dep
                    )));
                }
            }

            cap.enabled = true;
            Ok(())
        } else {
            Err(Error::CapabilityNotFound(name.to_string()))
        }
    }

    /// Disable a capability
    pub fn disable(&self, name: &str) -> Result<()> {
        if let Some(mut cap) = self.capabilities.get_mut(name) {
            cap.enabled = false;
            Ok(())
        } else {
            Err(Error::CapabilityNotFound(name.to_string()))
        }
    }

    /// Get capability
    pub fn get(&self, name: &str) -> Result<Capability> {
        self.capabilities
            .get(name)
            .map(|cap| cap.clone())
            .ok_or_else(|| Error::CapabilityNotFound(name.to_string()))
    }

    /// Update capability configuration
    pub fn set_config(&self, name: &str, config: serde_json::Value) -> Result<()> {
        if let Some(mut cap) = self.capabilities.get_mut(name) {
            cap.config = config;
            Ok(())
        } else {
            Err(Error::CapabilityNotFound(name.to_string()))
        }
    }

    /// Get all enabled capabilities
    pub fn enabled_capabilities(&self) -> Vec<Capability> {
        self.capabilities
            .iter()
            .filter(|entry| entry.enabled)
            .map(|entry| entry.clone())
            .collect()
    }

    /// Get all capabilities for a module
    pub fn for_module(&self, module: &str) -> Vec<Capability> {
        self.capabilities
            .iter()
            .filter(|entry| entry.module == module)
            .map(|entry| entry.clone())
            .collect()
    }

    /// List all capabilities
    pub fn list_all(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Capability count
    pub fn count(&self) -> usize {
        self.capabilities.len()
    }

    /// Get capability statistics
    pub fn stats(&self) -> CapabilityStats {
        let total = self.count();
        let enabled = self.enabled_capabilities().len();

        CapabilityStats {
            total,
            enabled,
            disabled: total - enabled,
        }
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityStats {
    pub total: usize,
    pub enabled: usize,
    pub disabled: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_creation() {
        let cap = Capability::new("compiler:rust", "ucc-compiler");
        assert_eq!(cap.name, "compiler:rust");
        assert_eq!(cap.module, "ucc-compiler");
        assert!(!cap.enabled);
    }

    #[test]
    fn test_capability_manager_register() {
        let manager = CapabilityManager::new();
        let cap = Capability::new("test-cap", "test-module");

        manager.register(cap).unwrap();
        assert!(manager.exists("test-cap"));
    }

    #[test]
    fn test_capability_manager_enable() {
        let manager = CapabilityManager::new();
        let cap = Capability::new("test-cap", "test-module");

        manager.register(cap).unwrap();
        manager.enable("test-cap").unwrap();

        assert!(manager.is_enabled("test-cap").unwrap());
    }

    #[test]
    fn test_capability_disable() {
        let manager = CapabilityManager::new();
        let cap = Capability::new("test-cap", "test-module");

        manager.register(cap).unwrap();
        manager.enable("test-cap").unwrap();
        manager.disable("test-cap").unwrap();

        assert!(!manager.is_enabled("test-cap").unwrap());
    }

    #[test]
    fn test_capability_dependencies() {
        let manager = CapabilityManager::new();

        let dep = Capability::new("compiler:base", "ucc");
        let cap = Capability::new("compiler:distributed", "ucc")
            .with_dependencies(vec!["compiler:base"]);

        manager.register(dep).unwrap();
        manager.register(cap).unwrap();

        // Try to enable dependent without enabling dependency
        let result = manager.enable("compiler:distributed");
        assert!(result.is_err());

        // Enable dependency first
        manager.enable("compiler:base").unwrap();
        manager.enable("compiler:distributed").unwrap();

        assert!(manager.is_enabled("compiler:distributed").unwrap());
    }

    #[test]
    fn test_capability_stats() {
        let manager = CapabilityManager::new();

        manager.register(Capability::new("cap1", "mod")).unwrap();
        manager.register(Capability::new("cap2", "mod")).unwrap();

        manager.enable("cap1").unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.enabled, 1);
        assert_eq!(stats.disabled, 1);
    }
}
