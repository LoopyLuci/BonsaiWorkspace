use crate::{CacheError, CacheResult, ReplicationConfig};
use dashmap::DashMap;
use std::sync::Arc;

pub struct ReplicationManager {
    nodes: Arc<DashMap<String, String>>,
}

impl ReplicationManager {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_node(&self, node_id: &str, node_addr: &str) -> CacheResult<()> {
        self.nodes.insert(node_id.to_string(), node_addr.to_string());
        Ok(())
    }

    pub async fn remove_node(&self, node_id: &str) -> CacheResult<()> {
        if self.nodes.remove(node_id).is_some() {
            Ok(())
        } else {
            Err(CacheError::ReplicationFailed)
        }
    }

    pub async fn replicate(&self, _config: &ReplicationConfig) -> CacheResult<()> {
        Ok(())
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for ReplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_node() {
        let rm = ReplicationManager::new();
        rm.add_node("node1", "127.0.0.1:6379").await.unwrap();
        assert_eq!(rm.node_count(), 1);
    }

    #[tokio::test]
    async fn test_remove_node() {
        let rm = ReplicationManager::new();
        rm.add_node("node1", "127.0.0.1:6379").await.unwrap();
        rm.remove_node("node1").await.unwrap();
        assert_eq!(rm.node_count(), 0);
    }
}
