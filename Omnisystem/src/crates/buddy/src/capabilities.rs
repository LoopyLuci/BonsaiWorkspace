use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct CapabilityRegistry {
    capabilities: Arc<DashMap<String, String>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        let registry = Self {
            capabilities: Arc::new(DashMap::new()),
        };
        registry.init_core_capabilities();
        registry
    }

    fn init_core_capabilities(&self) {
        let core = vec![
            ("iot_control", "Control IoT devices across all protocols"),
            ("search", "Search and retrieve information"),
            ("fabrication", "Manage 3D printers and fabrication devices"),
            ("agents", "Coordinate autonomous agents"),
            ("network", "Manage network infrastructure"),
            ("runtime", "Execute tasks and manage resources"),
            ("communication", "Route messages between modules"),
            ("security", "Encrypt and authorize operations"),
        ];

        for (name, desc) in core {
            let _ = self.register(name.to_string(), desc.to_string());
        }
    }

    pub fn register(&self, name: String, description: String) -> Result<()> {
        self.capabilities.insert(name, description);
        tracing::info!("Capability registered");
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.capabilities.get(name).map(|ref_| ref_.value().clone())
    }

    pub fn list(&self) -> Vec<(String, String)> {
        self.capabilities
            .iter()
            .map(|ref_| (ref_.key().clone(), ref_.value().clone()))
            .collect()
    }

    pub fn is_available(&self, name: &str) -> bool {
        self.capabilities.contains_key(name)
    }

    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_registry() {
        let registry = CapabilityRegistry::new();
        assert!(registry.capability_count() > 0);
        assert!(registry.is_available("iot_control"));
    }

    #[test]
    fn test_register_custom_capability() {
        let registry = CapabilityRegistry::new();
        registry.register("custom_ability".to_string(), "Custom capability".to_string()).unwrap();
        assert!(registry.is_available("custom_ability"));
    }
}
