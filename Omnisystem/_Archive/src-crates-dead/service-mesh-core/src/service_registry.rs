use crate::{EndpointId, MeshError, MeshResult, ServiceEndpoint, ServiceId, ServiceInstance, ServiceStatus};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ServiceRegistry {
    services: Arc<DashMap<String, ServiceInstance>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_service(&self, service: &ServiceInstance) -> MeshResult<()> {
        if service.endpoints.is_empty() {
            return Err(MeshError::InvalidConfiguration("Service must have at least one endpoint".to_string()));
        }

        self.services.insert(service.service_id.0.clone(), service.clone());
        Ok(())
    }

    pub async fn deregister_service(&self, service_id: &ServiceId) -> MeshResult<()> {
        self.services.remove(&service_id.0);
        Ok(())
    }

    pub async fn get_service(&self, service_id: &ServiceId) -> MeshResult<ServiceInstance> {
        self.services
            .get(&service_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| MeshError::ServiceNotFound(service_id.0.clone()))
    }

    pub async fn list_services(&self) -> MeshResult<Vec<ServiceInstance>> {
        Ok(self.services.iter().map(|entry| entry.value().clone()).collect())
    }

    pub async fn update_endpoint_status(
        &self,
        service_id: &ServiceId,
        endpoint_id: &EndpointId,
        healthy: bool,
    ) -> MeshResult<()> {
        if let Some(mut service) = self.services.get_mut(&service_id.0) {
            for endpoint in &mut service.endpoints {
                if endpoint.id == *endpoint_id {
                    endpoint.status = if healthy {
                        ServiceStatus::Healthy
                    } else {
                        ServiceStatus::Unhealthy
                    };

                    if healthy {
                        endpoint.success_count += 1;
                        endpoint.failure_count = endpoint.failure_count.saturating_sub(1);
                    } else {
                        endpoint.failure_count += 1;
                    }

                    let all_healthy = service.endpoints.iter().all(|e| e.status == ServiceStatus::Healthy);
                    service.status = if all_healthy {
                        ServiceStatus::Healthy
                    } else {
                        ServiceStatus::Degraded
                    };

                    return Ok(());
                }
            }
            Err(MeshError::EndpointUnavailable)
        } else {
            Err(MeshError::ServiceNotFound(service_id.0.clone()))
        }
    }

    pub async fn get_healthy_endpoints(&self, service_id: &ServiceId) -> MeshResult<Vec<ServiceEndpoint>> {
        self.services
            .get(&service_id.0)
            .map(|service| {
                service
                    .endpoints
                    .iter()
                    .filter(|e| e.status == ServiceStatus::Healthy)
                    .cloned()
                    .collect()
            })
            .ok_or_else(|| MeshError::ServiceNotFound(service_id.0.clone()))
    }

    pub fn service_count(&self) -> usize {
        self.services.len()
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
    use chrono::Utc;

    fn create_test_service(service_id: ServiceId) -> ServiceInstance {
        ServiceInstance {
            service_id: service_id.clone(),
            endpoints: vec![
                ServiceEndpoint {
                    id: EndpointId(uuid::Uuid::new_v4().to_string()),
                    address: "127.0.0.1".to_string(),
                    port: 8080,
                    weight: 100,
                    status: ServiceStatus::Healthy,
                    last_checked: Utc::now(),
                    failure_count: 0,
                    success_count: 0,
                },
                ServiceEndpoint {
                    id: EndpointId(uuid::Uuid::new_v4().to_string()),
                    address: "127.0.0.1".to_string(),
                    port: 8081,
                    weight: 100,
                    status: ServiceStatus::Healthy,
                    last_checked: Utc::now(),
                    failure_count: 0,
                    success_count: 0,
                },
            ],
            status: ServiceStatus::Healthy,
            metadata: Default::default(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_register_service() {
        let registry = ServiceRegistry::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());
        let service = create_test_service(service_id.clone());

        registry.register_service(&service).await.unwrap();
        assert_eq!(registry.service_count(), 1);
    }

    #[tokio::test]
    async fn test_get_service() {
        let registry = ServiceRegistry::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());
        let service = create_test_service(service_id.clone());

        registry.register_service(&service).await.unwrap();
        let retrieved = registry.get_service(&service_id).await.unwrap();

        assert_eq!(retrieved.service_id, service_id);
        assert_eq!(retrieved.endpoints.len(), 2);
    }

    #[tokio::test]
    async fn test_deregister_service() {
        let registry = ServiceRegistry::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());
        let service = create_test_service(service_id.clone());

        registry.register_service(&service).await.unwrap();
        registry.deregister_service(&service_id).await.unwrap();

        assert_eq!(registry.service_count(), 0);
    }

    #[tokio::test]
    async fn test_get_healthy_endpoints() {
        let registry = ServiceRegistry::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());
        let service = create_test_service(service_id.clone());

        registry.register_service(&service).await.unwrap();
        let healthy = registry.get_healthy_endpoints(&service_id).await.unwrap();

        assert_eq!(healthy.len(), 2);
    }

    #[tokio::test]
    async fn test_update_endpoint_status() {
        let registry = ServiceRegistry::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());
        let service = create_test_service(service_id.clone());
        let endpoint_id = service.endpoints[0].id.clone();

        registry.register_service(&service).await.unwrap();
        registry
            .update_endpoint_status(&service_id, &endpoint_id, false)
            .await
            .unwrap();

        let updated = registry.get_service(&service_id).await.unwrap();
        assert_eq!(updated.status, ServiceStatus::Degraded);
    }

    #[tokio::test]
    async fn test_empty_endpoints_rejected() {
        let registry = ServiceRegistry::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        let mut service = create_test_service(service_id);
        service.endpoints.clear();

        let result = registry.register_service(&service).await;
        assert!(result.is_err());
    }
}
