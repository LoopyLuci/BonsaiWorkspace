use crate::{DomainName, VirtualHost, VirtualHostId, WebError, WebResult};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct VirtualHostRouter {
    routes: Arc<DashMap<String, VirtualHostId>>,
    backends: Arc<DashMap<String, Vec<String>>>,
    round_robin_counters: Arc<DashMap<String, AtomicUsize>>,
}

impl VirtualHostRouter {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(DashMap::new()),
            backends: Arc::new(DashMap::new()),
            round_robin_counters: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_route(
        &self,
        domain: &DomainName,
        vhost_id: &VirtualHostId,
    ) -> WebResult<()> {
        self.routes
            .insert(domain.0.clone(), vhost_id.clone());
        Ok(())
    }

    pub async fn register_backends(
        &self,
        vhost_id: &VirtualHostId,
        backends: Vec<String>,
    ) -> WebResult<()> {
        if backends.is_empty() {
            return Err(WebError::ConfigurationError(
                "At least one backend required".to_string(),
            ));
        }

        self.backends.insert(vhost_id.0.to_string(), backends);
        Ok(())
    }

    pub async fn resolve_host(&self, domain: &DomainName) -> WebResult<VirtualHostId> {
        self.routes
            .get(&domain.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| WebError::VirtualHostNotFound(domain.0.clone()))
    }

    pub async fn get_next_backend(
        &self,
        vhost_id: &VirtualHostId,
    ) -> WebResult<String> {
        let backends = self
            .backends
            .get(&vhost_id.0.to_string())
            .ok_or_else(|| {
                WebError::ConfigurationError(format!(
                    "No backends configured for vhost {}",
                    vhost_id.0
                ))
            })?;

        let counter = self
            .round_robin_counters
            .entry(vhost_id.0.to_string())
            .or_insert_with(|| AtomicUsize::new(0));

        let index = counter.fetch_add(1, Ordering::Relaxed) % backends.len();
        Ok(backends[index].clone())
    }

    pub async fn get_all_backends(&self, vhost_id: &VirtualHostId) -> WebResult<Vec<String>> {
        self.backends
            .get(&vhost_id.0.to_string())
            .map(|entry| entry.clone())
            .ok_or_else(|| {
                WebError::ConfigurationError(format!(
                    "No backends configured for vhost {}",
                    vhost_id.0
                ))
            })
    }

    pub async fn remove_route(&self, domain: &DomainName) -> WebResult<()> {
        self.routes.remove(&domain.0);
        Ok(())
    }

    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    pub fn backend_count(&self, vhost_id: &VirtualHostId) -> Option<usize> {
        self.backends
            .get(&vhost_id.0.to_string())
            .map(|entry| entry.len())
    }
}

impl Default for VirtualHostRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_register_and_resolve_route() {
        let router = VirtualHostRouter::new();
        let domain = DomainName("example.com".to_string());
        let vhost_id = VirtualHostId(Uuid::new_v4());

        router.register_route(&domain, &vhost_id).await.unwrap();

        let resolved = router.resolve_host(&domain).await.unwrap();
        assert_eq!(resolved, vhost_id);
    }

    #[tokio::test]
    async fn test_round_robin_backend_selection() {
        let router = VirtualHostRouter::new();
        let vhost_id = VirtualHostId(Uuid::new_v4());
        let backends = vec!["backend1".to_string(), "backend2".to_string(), "backend3".to_string()];

        router.register_backends(&vhost_id, backends).await.unwrap();

        let first = router.get_next_backend(&vhost_id).await.unwrap();
        let second = router.get_next_backend(&vhost_id).await.unwrap();
        let third = router.get_next_backend(&vhost_id).await.unwrap();

        assert_ne!(first, second);
        assert_ne!(second, third);
    }

    #[tokio::test]
    async fn test_no_backends_error() {
        let router = VirtualHostRouter::new();
        let vhost_id = VirtualHostId(Uuid::new_v4());

        let result = router
            .register_backends(&vhost_id, vec![])
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_backends() {
        let router = VirtualHostRouter::new();
        let vhost_id = VirtualHostId(Uuid::new_v4());
        let backends = vec!["backend1".to_string(), "backend2".to_string()];

        router.register_backends(&vhost_id, backends.clone()).await.unwrap();

        let retrieved = router.get_all_backends(&vhost_id).await.unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_route() {
        let router = VirtualHostRouter::new();
        let domain = DomainName("example.com".to_string());
        let vhost_id = VirtualHostId(Uuid::new_v4());

        router.register_route(&domain, &vhost_id).await.unwrap();
        assert_eq!(router.route_count(), 1);

        router.remove_route(&domain).await.unwrap();
        assert_eq!(router.route_count(), 0);
    }
}
