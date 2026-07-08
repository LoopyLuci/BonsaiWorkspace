use crate::{MeshService, SidecarProxy, ServiceEndpoint, MeshConfig, MeshError, MeshResult, ProxyStatus};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct MeshManager {
    services: Arc<DashMap<Uuid, MeshService>>,
    sidecars: Arc<DashMap<Uuid, SidecarProxy>>,
    endpoints: Arc<DashMap<Uuid, ServiceEndpoint>>,
}

impl MeshManager {
    pub fn new() -> Self {
        Self {
            services: Arc::new(DashMap::new()),
            sidecars: Arc::new(DashMap::new()),
            endpoints: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_service(&self, service: &MeshService) -> MeshResult<()> {
        self.services.insert(service.service_id, service.clone());
        Ok(())
    }

    pub async fn get_service(&self, service_id: Uuid) -> MeshResult<MeshService> {
        self.services
            .get(&service_id)
            .map(|s| s.clone())
            .ok_or(MeshError::ServiceNotFound)
    }

    pub async fn register_sidecar(&self, sidecar: &SidecarProxy) -> MeshResult<()> {
        self.sidecars.insert(sidecar.proxy_id, sidecar.clone());
        Ok(())
    }

    pub async fn update_sidecar_status(&self, proxy_id: Uuid, status: ProxyStatus) -> MeshResult<()> {
        if let Some(mut sidecar) = self.sidecars.get_mut(&proxy_id) {
            sidecar.status = status;
            Ok(())
        } else {
            Err(MeshError::SidecarNotFound)
        }
    }

    pub async fn register_endpoint(&self, endpoint: &ServiceEndpoint) -> MeshResult<()> {
        self.endpoints.insert(endpoint.endpoint_id, endpoint.clone());
        Ok(())
    }

    pub async fn get_service_endpoints(&self, service_id: Uuid) -> MeshResult<Vec<ServiceEndpoint>> {
        let mut endpoints = Vec::new();

        for entry in self.endpoints.iter() {
            if entry.value().service_id == service_id {
                endpoints.push(entry.value().clone());
            }
        }

        Ok(endpoints)
    }

    pub async fn discover_services(&self, namespace: &str) -> MeshResult<Vec<MeshService>> {
        let mut discovered = Vec::new();

        for entry in self.services.iter() {
            if entry.value().namespace == namespace {
                discovered.push(entry.value().clone());
            }
        }

        Ok(discovered)
    }

    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    pub fn sidecar_count(&self) -> usize {
        self.sidecars.len()
    }
}

impl Default for MeshManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Protocol;

    #[tokio::test]
    async fn test_register_service() {
        let manager = MeshManager::new();
        let service = MeshService {
            service_id: Uuid::new_v4(),
            name: "api-service".to_string(),
            namespace: "default".to_string(),
            port: 8080,
            protocol: Protocol::HTTP,
            version: "1.0.0".to_string(),
        };

        manager.register_service(&service).await.unwrap();
        assert_eq!(manager.service_count(), 1);
    }

    #[tokio::test]
    async fn test_register_sidecar() {
        let manager = MeshManager::new();
        let service_id = Uuid::new_v4();
        let sidecar = SidecarProxy {
            proxy_id: Uuid::new_v4(),
            service_id,
            pod_ip: "10.0.0.1".to_string(),
            proxy_port: 15000,
            admin_port: 15001,
            status: ProxyStatus::Ready,
        };

        manager.register_sidecar(&sidecar).await.unwrap();
        assert_eq!(manager.sidecar_count(), 1);
    }

    #[tokio::test]
    async fn test_discover_services() {
        let manager = MeshManager::new();
        let service = MeshService {
            service_id: Uuid::new_v4(),
            name: "web".to_string(),
            namespace: "prod".to_string(),
            port: 80,
            protocol: Protocol::HTTP,
            version: "2.0.0".to_string(),
        };

        manager.register_service(&service).await.unwrap();

        let services = manager.discover_services("prod").await.unwrap();
        assert_eq!(services.len(), 1);
    }

    #[tokio::test]
    async fn test_register_endpoint() {
        let manager = MeshManager::new();
        let service_id = Uuid::new_v4();
        let endpoint = ServiceEndpoint {
            endpoint_id: Uuid::new_v4(),
            service_id,
            address: "10.0.0.2".to_string(),
            port: 8080,
            weight: 100,
            healthy: true,
        };

        manager.register_endpoint(&endpoint).await.unwrap();

        let endpoints = manager.get_service_endpoints(service_id).await.unwrap();
        assert_eq!(endpoints.len(), 1);
    }
}
