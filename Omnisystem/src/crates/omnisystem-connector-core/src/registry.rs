use crate::{ConnectorError, Result, ConnectorId};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ConnectorRegistry {
    connectors: Arc<DashMap<ConnectorId, Arc<ConnectorMetadata>>>,
}

#[derive(Debug, Clone)]
pub struct ConnectorMetadata {
    pub id: ConnectorId,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub name: Option<String>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self {
            connectors: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, id: ConnectorId) -> Result<()> {
        if self.connectors.contains_key(&id) {
            return Err(ConnectorError::AlreadyExists(id.to_string()));
        }

        let metadata = Arc::new(ConnectorMetadata {
            id,
            created_at: chrono::Utc::now(),
            name: None,
        });

        self.connectors.insert(id, metadata);
        tracing::info!("Registered connector: {}", id);
        Ok(())
    }

    pub fn register_named(&self, id: ConnectorId, name: String) -> Result<()> {
        if self.connectors.contains_key(&id) {
            return Err(ConnectorError::AlreadyExists(id.to_string()));
        }

        let metadata = Arc::new(ConnectorMetadata {
            id,
            created_at: chrono::Utc::now(),
            name: Some(name),
        });

        self.connectors.insert(id, metadata);
        tracing::info!("Registered connector: {}", id);
        Ok(())
    }

    pub fn unregister(&self, id: ConnectorId) -> Result<()> {
        self.connectors
            .remove(&id)
            .ok_or(ConnectorError::NotFound(id.to_string()))?;

        tracing::info!("Unregistered connector: {}", id);
        Ok(())
    }

    pub fn exists(&self, id: ConnectorId) -> bool {
        self.connectors.contains_key(&id)
    }

    pub fn get(&self, id: ConnectorId) -> Option<ConnectorMetadata> {
        self.connectors.get(&id).map(|m| m.value().as_ref().clone())
    }

    pub fn list_all(&self) -> Vec<ConnectorId> {
        self.connectors.iter().map(|ref_| *ref_.key()).collect()
    }

    pub fn count(&self) -> usize {
        self.connectors.len()
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = ConnectorRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_register() {
        let registry = ConnectorRegistry::new();
        let id = ConnectorId::new();

        assert!(registry.register(id).is_ok());
        assert!(registry.exists(id));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_registry_duplicate() {
        let registry = ConnectorRegistry::new();
        let id = ConnectorId::new();

        registry.register(id).unwrap();
        assert!(registry.register(id).is_err());
    }

    #[test]
    fn test_registry_unregister() {
        let registry = ConnectorRegistry::new();
        let id = ConnectorId::new();

        registry.register(id).unwrap();
        assert!(registry.unregister(id).is_ok());
        assert!(!registry.exists(id));
    }

    #[test]
    fn test_registry_list() {
        let registry = ConnectorRegistry::new();
        let id1 = ConnectorId::new();
        let id2 = ConnectorId::new();

        registry.register(id1).unwrap();
        registry.register(id2).unwrap();

        let list = registry.list_all();
        assert_eq!(list.len(), 2);
    }
}
