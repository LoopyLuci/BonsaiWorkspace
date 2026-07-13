use crate::{ContainerError, ContainerResult, RegistryConfig, RegistryUrl};
use dashmap::DashMap;
use std::sync::Arc;

pub struct RegistryClient {
    registries: Arc<DashMap<String, RegistryConfig>>,
}

impl RegistryClient {
    pub fn new() -> Self {
        Self {
            registries: Arc::new(DashMap::new()),
        }
    }

    pub fn registry_count(&self) -> usize {
        self.registries.len()
    }

    pub async fn register_registry(&self, config: &RegistryConfig) -> ContainerResult<()> {
        if self.registries.contains_key(&config.url.0) {
            return Err(ContainerError::RegistryError(format!(
                "Registry already registered: {}",
                config.url.0
            )));
        }

        self.registries.insert(config.url.0.clone(), config.clone());
        Ok(())
    }

    pub async fn get_registry(&self, url: &RegistryUrl) -> ContainerResult<RegistryConfig> {
        self.registries
            .get(&url.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| ContainerError::RegistryError(format!("Registry not found: {}", url.0)))
    }

    pub async fn list_registries(&self) -> ContainerResult<Vec<RegistryConfig>> {
        Ok(self
            .registries
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn remove_registry(&self, url: &RegistryUrl) -> ContainerResult<()> {
        if self.registries.remove(&url.0).is_some() {
            Ok(())
        } else {
            Err(ContainerError::RegistryError(format!(
                "Registry not found: {}",
                url.0
            )))
        }
    }

    pub async fn test_registry_connection(&self, url: &RegistryUrl) -> ContainerResult<bool> {
        if self.registries.contains_key(&url.0) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn authenticate_registry(
        &self,
        url: &RegistryUrl,
        username: &str,
        password: &str,
    ) -> ContainerResult<bool> {
        if let Some(registry) = self.registries.get(&url.0) {
            let auth_valid = registry.username.as_ref().map_or(false, |u| u == username)
                && registry.password.as_ref().map_or(false, |p| p == password);
            Ok(auth_valid)
        } else {
            Err(ContainerError::RegistryError(format!(
                "Registry not found: {}",
                url.0
            )))
        }
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_registry() {
        let client = RegistryClient::new();
        let config = RegistryConfig {
            url: RegistryUrl("docker.io".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            insecure: false,
        };

        client.register_registry(&config).await.unwrap();
        assert_eq!(client.registry_count(), 1);
    }

    #[tokio::test]
    async fn test_get_registry() {
        let client = RegistryClient::new();
        let config = RegistryConfig {
            url: RegistryUrl("docker.io".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            insecure: false,
        };

        client.register_registry(&config).await.unwrap();
        let retrieved = client.get_registry(&config.url).await.unwrap();
        assert_eq!(retrieved.url.0, "docker.io");
    }

    #[tokio::test]
    async fn test_list_registries() {
        let client = RegistryClient::new();

        let config1 = RegistryConfig {
            url: RegistryUrl("docker.io".to_string()),
            username: None,
            password: None,
            insecure: false,
        };

        let config2 = RegistryConfig {
            url: RegistryUrl("quay.io".to_string()),
            username: None,
            password: None,
            insecure: false,
        };

        client.register_registry(&config1).await.unwrap();
        client.register_registry(&config2).await.unwrap();

        let registries = client.list_registries().await.unwrap();
        assert_eq!(registries.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_registry() {
        let client = RegistryClient::new();
        let config = RegistryConfig {
            url: RegistryUrl("docker.io".to_string()),
            username: None,
            password: None,
            insecure: false,
        };

        client.register_registry(&config).await.unwrap();
        assert_eq!(client.registry_count(), 1);

        client.remove_registry(&config.url).await.unwrap();
        assert_eq!(client.registry_count(), 0);
    }

    #[tokio::test]
    async fn test_test_connection() {
        let client = RegistryClient::new();
        let config = RegistryConfig {
            url: RegistryUrl("docker.io".to_string()),
            username: None,
            password: None,
            insecure: false,
        };

        client.register_registry(&config).await.unwrap();
        let connected = client.test_registry_connection(&config.url).await.unwrap();
        assert!(connected);
    }

    #[tokio::test]
    async fn test_authenticate_registry() {
        let client = RegistryClient::new();
        let config = RegistryConfig {
            url: RegistryUrl("docker.io".to_string()),
            username: Some("testuser".to_string()),
            password: Some("testpass".to_string()),
            insecure: false,
        };

        client.register_registry(&config).await.unwrap();
        let authenticated = client
            .authenticate_registry(&config.url, "testuser", "testpass")
            .await
            .unwrap();
        assert!(authenticated);
    }
}
