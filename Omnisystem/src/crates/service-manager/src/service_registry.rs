//! Service manifest registry (integration point with UMS)

use crate::error::{Result, SLMError};
use crate::types::ServiceManifest;
use dashmap::DashMap;
use log::debug;
use std::sync::Arc;

/// Service registry backed by manifests from UMS
pub struct ServiceRegistry {
    /// Cached manifests (service_name -> ServiceManifest)
    manifests: Arc<DashMap<String, ServiceManifest>>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            manifests: Arc::new(DashMap::new()),
        }
    }

    /// Register a service manifest (called when UMS module is loaded)
    pub fn register_service(&self, manifest: ServiceManifest) -> Result<()> {
        debug!("Registering service: {}", manifest.name);

        // Validate manifest
        self.validate_manifest(&manifest)?;

        self.manifests.insert(manifest.name.clone(), manifest);
        Ok(())
    }

    /// Get a service manifest by name
    pub fn get_service(&self, service_name: &str) -> Result<ServiceManifest> {
        debug!("Looking up service: {}", service_name);

        self.manifests
            .get(service_name)
            .map(|entry| entry.clone())
            .ok_or_else(|| SLMError::ServiceNotFound(service_name.to_string()))
    }

    /// List all registered services
    pub fn list_services(&self) -> Vec<String> {
        self.manifests
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Remove a service manifest (e.g., on UMS module deletion)
    pub fn unregister_service(&self, service_name: &str) -> Result<()> {
        debug!("Unregistering service: {}", service_name);

        self.manifests
            .remove(service_name)
            .ok_or_else(|| SLMError::ServiceNotFound(service_name.to_string()))?;

        Ok(())
    }

    /// Check if a service is registered
    pub fn has_service(&self, service_name: &str) -> bool {
        self.manifests.contains_key(service_name)
    }

    /// Validate a manifest
    fn validate_manifest(&self, manifest: &ServiceManifest) -> Result<()> {
        if manifest.name.is_empty() {
            return Err(SLMError::ManifestError(
                "Service name cannot be empty".to_string(),
            ));
        }

        if manifest.binary_hash.is_empty() {
            return Err(SLMError::ManifestError(
                "Binary hash cannot be empty".to_string(),
            ));
        }

        if manifest.idle_timeout_secs == 0 {
            return Err(SLMError::ManifestError(
                "Idle timeout must be > 0".to_string(),
            ));
        }

        if manifest.quota.memory_mb == 0 {
            return Err(SLMError::ManifestError(
                "Memory quota must be > 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest(name: &str) -> ServiceManifest {
        ServiceManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            binary_hash: "abc123".to_string(),
            capabilities_required: vec!["USB".to_string()],
            quota: Default::default(),
            idle_timeout_secs: 300,
            archive_after_hours: 24,
            heartbeat_interval_secs: 10,
            heartbeat_timeout_secs: 5,
            signature: "sig123".to_string(),
        }
    }

    #[test]
    fn test_register_and_get_service() {
        let registry = ServiceRegistry::new();
        let manifest = sample_manifest("fax");

        assert!(registry.register_service(manifest.clone()).is_ok());
        assert!(registry.has_service("fax"));

        let retrieved = registry.get_service("fax").unwrap();
        assert_eq!(retrieved.name, "fax");
    }

    #[test]
    fn test_get_nonexistent_service() {
        let registry = ServiceRegistry::new();
        let result = registry.get_service("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_register_multiple_services() {
        let registry = ServiceRegistry::new();
        registry.register_service(sample_manifest("fax")).unwrap();
        registry.register_service(sample_manifest("scanner")).unwrap();
        registry.register_service(sample_manifest("printer")).unwrap();

        let services = registry.list_services();
        assert_eq!(services.len(), 3);
        assert!(services.contains(&"fax".to_string()));
    }

    #[test]
    fn test_unregister_service() {
        let registry = ServiceRegistry::new();
        registry.register_service(sample_manifest("fax")).unwrap();

        assert!(registry.has_service("fax"));
        assert!(registry.unregister_service("fax").is_ok());
        assert!(!registry.has_service("fax"));
    }

    #[test]
    fn test_invalid_manifest_empty_name() {
        let registry = ServiceRegistry::new();
        let invalid = ServiceManifest {
            name: String::new(),
            ..sample_manifest("test")
        };

        let result = registry.register_service(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_manifest_zero_memory() {
        let registry = ServiceRegistry::new();
        let mut invalid = sample_manifest("test");
        invalid.quota.memory_mb = 0;

        let result = registry.register_service(invalid);
        assert!(result.is_err());
    }
}
