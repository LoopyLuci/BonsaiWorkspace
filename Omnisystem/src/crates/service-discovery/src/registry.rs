use crate::{DiscoveryError, DiscoveryResult, ServiceInstance, ServiceRegistry, ServiceStatus};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::collections::HashMap;

pub struct ServiceRegistryImpl {
    services: Arc<DashMap<String, Vec<ServiceInstance>>>,
}

impl ServiceRegistryImpl {
    pub fn new() -> Self {
        Self {
            services: Arc::new(DashMap::new()),
        }
    }

    pub async fn register(
        &self,
        instance: &ServiceInstance,
    ) -> DiscoveryResult<()> {
        let mut instances = self
            .services
            .entry(instance.service_name.clone())
            .or_insert_with(Vec::new);

        if instances.iter().any(|i| i.instance_id == instance.instance_id) {
            return Err(DiscoveryError::ServiceAlreadyRegistered);
        }

        instances.push(instance.clone());
        Ok(())
    }

    pub async fn deregister(&self, service_name: &str, instance_id: &str) -> DiscoveryResult<()> {
        if let Some(mut instances) = self.services.get_mut(service_name) {
            instances.retain(|i| i.instance_id != instance_id);
            Ok(())
        } else {
            Err(DiscoveryError::ServiceNotFound)
        }
    }

    pub async fn get_instances(&self, service_name: &str) -> DiscoveryResult<Vec<ServiceInstance>> {
        if let Some(instances) = self.services.get(service_name) {
            Ok(instances
                .iter()
                .filter(|i| i.status == ServiceStatus::Healthy)
                .cloned()
                .collect())
        } else {
            Err(DiscoveryError::ServiceNotFound)
        }
    }

    pub async fn get_all_instances(&self, service_name: &str) -> DiscoveryResult<Vec<ServiceInstance>> {
        if let Some(instances) = self.services.get(service_name) {
            Ok(instances.iter().cloned().collect())
        } else {
            Err(DiscoveryError::ServiceNotFound)
        }
    }

    pub async fn update_status(
        &self,
        service_name: &str,
        instance_id: &str,
        status: ServiceStatus,
    ) -> DiscoveryResult<()> {
        if let Some(mut instances) = self.services.get_mut(service_name) {
            for instance in instances.iter_mut() {
                if instance.instance_id == instance_id {
                    instance.status = status;
                    instance.last_heartbeat = Utc::now();
                    return Ok(());
                }
            }
            Err(DiscoveryError::ServiceNotFound)
        } else {
            Err(DiscoveryError::ServiceNotFound)
        }
    }

    pub async fn get_registry(&self, service_name: &str) -> DiscoveryResult<ServiceRegistry> {
        if let Some(instances) = self.services.get(service_name) {
            let healthy_count = instances.iter().filter(|i| i.status == ServiceStatus::Healthy).count();
            Ok(ServiceRegistry {
                service_name: service_name.to_string(),
                instances: instances.iter().cloned().collect(),
                total_instances: instances.len() as u32,
                healthy_instances: healthy_count as u32,
            })
        } else {
            Err(DiscoveryError::ServiceNotFound)
        }
    }

    pub fn service_count(&self) -> usize {
        self.services.len()
    }
}

impl Default for ServiceRegistryImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_service() {
        let registry = ServiceRegistryImpl::new();
        let instance = ServiceInstance {
            instance_id: "instance-1".to_string(),
            service_name: "api".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            status: ServiceStatus::Healthy,
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            metadata: HashMap::new(),
        };

        registry.register(&instance).await.unwrap();
        assert_eq!(registry.service_count(), 1);
    }

    #[tokio::test]
    async fn test_get_instances() {
        let registry = ServiceRegistryImpl::new();
        let instance = ServiceInstance {
            instance_id: "instance-1".to_string(),
            service_name: "api".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            status: ServiceStatus::Healthy,
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            metadata: HashMap::new(),
        };

        registry.register(&instance).await.unwrap();
        let instances = registry.get_instances("api").await.unwrap();

        assert_eq!(instances.len(), 1);
    }

    #[tokio::test]
    async fn test_deregister_service() {
        let registry = ServiceRegistryImpl::new();
        let instance = ServiceInstance {
            instance_id: "instance-1".to_string(),
            service_name: "api".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            status: ServiceStatus::Healthy,
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            metadata: HashMap::new(),
        };

        registry.register(&instance).await.unwrap();
        registry.deregister("api", "instance-1").await.unwrap();
        let result = registry.get_instances("api").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_status() {
        let registry = ServiceRegistryImpl::new();
        let instance = ServiceInstance {
            instance_id: "instance-1".to_string(),
            service_name: "api".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            status: ServiceStatus::Healthy,
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            metadata: HashMap::new(),
        };

        registry.register(&instance).await.unwrap();
        registry
            .update_status("api", "instance-1", ServiceStatus::Unhealthy)
            .await
            .unwrap();

        let instances = registry.get_all_instances("api").await.unwrap();
        assert_eq!(instances[0].status, ServiceStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_get_registry() {
        let registry = ServiceRegistryImpl::new();
        let instance = ServiceInstance {
            instance_id: "instance-1".to_string(),
            service_name: "api".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            status: ServiceStatus::Healthy,
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            metadata: HashMap::new(),
        };

        registry.register(&instance).await.unwrap();
        let service_reg = registry.get_registry("api").await.unwrap();

        assert_eq!(service_reg.total_instances, 1);
        assert_eq!(service_reg.healthy_instances, 1);
    }
}
