use crate::{
    DomainName, HttpRequest, HttpResponse, HttpVersion, VirtualHost, VirtualHostId,
    WebHostingManager, WebResult, WebError,
};
use dashmap::DashMap;
use std::sync::Arc;
use chrono::Utc;

pub struct WebServer {
    vhosts: Arc<DashMap<String, VirtualHost>>,
    vhost_by_domain: Arc<DashMap<String, VirtualHostId>>,
    total_requests: Arc<std::sync::atomic::AtomicU64>,
    total_bytes_served: Arc<std::sync::atomic::AtomicU64>,
}

impl WebServer {
    pub fn new() -> Self {
        Self {
            vhosts: Arc::new(DashMap::new()),
            vhost_by_domain: Arc::new(DashMap::new()),
            total_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_bytes_served: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn vhost_count(&self) -> usize {
        self.vhosts.len()
    }

    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn total_bytes_served(&self) -> u64 {
        self.total_bytes_served.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for WebServer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl WebHostingManager for WebServer {
    async fn create_virtual_host(&self, vhost: VirtualHost) -> WebResult<VirtualHost> {
        let vhost_id_str = vhost.id.0.to_string();
        let domain_str = vhost.domain.0.clone();

        if self.vhost_by_domain.contains_key(&domain_str) {
            return Err(WebError::VirtualHostAlreadyExists(domain_str));
        }

        self.vhosts.insert(vhost_id_str, vhost.clone());
        self.vhost_by_domain.insert(domain_str, vhost.id.clone());

        Ok(vhost)
    }

    async fn delete_virtual_host(&self, id: &VirtualHostId) -> WebResult<()> {
        if let Some((_, vhost)) = self.vhosts.remove(&id.0.to_string()) {
            self.vhost_by_domain.remove(&vhost.domain.0);
            Ok(())
        } else {
            Err(WebError::VirtualHostNotFound(id.0.to_string()))
        }
    }

    async fn get_virtual_host(&self, id: &VirtualHostId) -> WebResult<VirtualHost> {
        self.vhosts
            .get(&id.0.to_string())
            .map(|entry| entry.clone())
            .ok_or_else(|| WebError::VirtualHostNotFound(id.0.to_string()))
    }

    async fn get_virtual_host_by_domain(&self, domain: &DomainName) -> WebResult<VirtualHost> {
        if let Some(vhost_id) = self.vhost_by_domain.get(&domain.0) {
            self.get_virtual_host(&vhost_id).await
        } else {
            Err(WebError::VirtualHostNotFound(domain.0.clone()))
        }
    }

    async fn list_virtual_hosts(&self) -> WebResult<Vec<VirtualHost>> {
        Ok(self.vhosts.iter().map(|entry| entry.value().clone()).collect())
    }

    async fn update_virtual_host(
        &self,
        id: &VirtualHostId,
        mut vhost: VirtualHost,
    ) -> WebResult<()> {
        if !self.vhosts.contains_key(&id.0.to_string()) {
            return Err(WebError::VirtualHostNotFound(id.0.to_string()));
        }

        vhost.updated_at = Utc::now();
        self.vhosts.insert(id.0.to_string(), vhost);
        Ok(())
    }

    async fn add_domain_alias(
        &self,
        id: &VirtualHostId,
        alias: DomainName,
    ) -> WebResult<()> {
        if let Some(mut vhost) = self.vhosts.get_mut(&id.0.to_string()) {
            if !vhost.aliases.contains(&alias) {
                vhost.aliases.push(alias.clone());
                self.vhost_by_domain.insert(alias.0, id.clone());
            }
            Ok(())
        } else {
            Err(WebError::VirtualHostNotFound(id.0.to_string()))
        }
    }

    async fn remove_domain_alias(
        &self,
        id: &VirtualHostId,
        alias: &DomainName,
    ) -> WebResult<()> {
        if let Some(mut vhost) = self.vhosts.get_mut(&id.0.to_string()) {
            vhost.aliases.retain(|a| a != alias);
            self.vhost_by_domain.remove(&alias.0);
            Ok(())
        } else {
            Err(WebError::VirtualHostNotFound(id.0.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_virtual_host() {
        let server = WebServer::new();
        let domain = DomainName("example.com".to_string());
        let vhost = VirtualHost::new(domain, "/var/www".to_string());

        let result = server.create_virtual_host(vhost.clone()).await;
        assert!(result.is_ok());
        assert_eq!(server.vhost_count(), 1);
    }

    #[tokio::test]
    async fn test_duplicate_virtual_host() {
        let server = WebServer::new();
        let domain = DomainName("example.com".to_string());
        let vhost = VirtualHost::new(domain, "/var/www".to_string());

        server.create_virtual_host(vhost.clone()).await.unwrap();
        let result = server.create_virtual_host(vhost).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_virtual_host_by_domain() {
        let server = WebServer::new();
        let domain = DomainName("example.com".to_string());
        let vhost = VirtualHost::new(domain.clone(), "/var/www".to_string());
        let vhost_id = vhost.id.clone();

        server.create_virtual_host(vhost).await.unwrap();

        let retrieved = server.get_virtual_host_by_domain(&domain).await.unwrap();
        assert_eq!(retrieved.id, vhost_id);
    }

    #[tokio::test]
    async fn test_add_domain_alias() {
        let server = WebServer::new();
        let domain = DomainName("example.com".to_string());
        let alias = DomainName("www.example.com".to_string());
        let vhost = VirtualHost::new(domain, "/var/www".to_string());
        let vhost_id = vhost.id.clone();

        server.create_virtual_host(vhost).await.unwrap();
        server.add_domain_alias(&vhost_id, alias.clone()).await.unwrap();

        let retrieved = server.get_virtual_host_by_domain(&alias).await.unwrap();
        assert!(retrieved.aliases.contains(&alias));
    }

    #[tokio::test]
    async fn test_delete_virtual_host() {
        let server = WebServer::new();
        let domain = DomainName("example.com".to_string());
        let vhost = VirtualHost::new(domain, "/var/www".to_string());
        let vhost_id = vhost.id.clone();

        server.create_virtual_host(vhost).await.unwrap();
        assert_eq!(server.vhost_count(), 1);

        server.delete_virtual_host(&vhost_id).await.unwrap();
        assert_eq!(server.vhost_count(), 0);
    }

    #[tokio::test]
    async fn test_list_virtual_hosts() {
        let server = WebServer::new();

        for i in 0..3 {
            let domain = DomainName(format!("example{}.com", i));
            let vhost = VirtualHost::new(domain, "/var/www".to_string());
            server.create_virtual_host(vhost).await.unwrap();
        }

        let vhosts = server.list_virtual_hosts().await.unwrap();
        assert_eq!(vhosts.len(), 3);
    }
}
