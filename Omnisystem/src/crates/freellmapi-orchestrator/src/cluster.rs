use crate::node::{Node, NodeStatus};
use dashmap::DashMap;
use std::sync::Arc;
use anyhow::Result;

pub struct ClusterManager {
    nodes: Arc<DashMap<String, Node>>,
}

impl ClusterManager {
    pub fn new() -> Self {
        ClusterManager {
            nodes: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_node(&self, node_id: &str, address: &str, capacity: u32) -> Result<()> {
        let node = Node::new(node_id.to_string(), address.to_string(), capacity);
        self.nodes.insert(node_id.to_string(), node);
        Ok(())
    }

    pub async fn select_node(&self) -> Result<Option<Node>> {
        // Round-robin selection of healthy nodes with available capacity
        let mut best_node = None;
        let mut best_available = 0;

        for entry in self.nodes.iter() {
            let node = entry.value();
            if node.is_healthy() && node.available_capacity() > best_available {
                best_available = node.available_capacity();
                best_node = Some(node.clone());
            }
        }

        Ok(best_node)
    }

    pub async fn mark_healthy(&self, node_id: &str) -> Result<()> {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.status = NodeStatus::Healthy;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            node.last_heartbeat = now;
        }
        Ok(())
    }

    pub async fn mark_unhealthy(&self, node_id: &str) -> Result<()> {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.status = NodeStatus::Unhealthy;
        }
        Ok(())
    }

    pub async fn get_all_nodes(&self) -> Result<Vec<Node>> {
        Ok(self.nodes.iter().map(|r| r.value().clone()).collect())
    }

    pub async fn get_healthy_nodes(&self) -> Result<Vec<Node>> {
        Ok(self.nodes
            .iter()
            .filter(|r| r.value().is_healthy())
            .map(|r| r.value().clone())
            .collect())
    }
}

impl Default for ClusterManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_select() {
        let manager = ClusterManager::new();
        manager.register_node("node1", "127.0.0.1:8000", 100).await.unwrap();
        manager.register_node("node2", "127.0.0.1:8001", 100).await.unwrap();

        let selected = manager.select_node().await.unwrap();
        assert!(selected.is_some());
    }

    #[tokio::test]
    async fn test_mark_unhealthy() {
        let manager = ClusterManager::new();
        manager.register_node("node1", "127.0.0.1:8000", 100).await.unwrap();
        manager.mark_unhealthy("node1").await.unwrap();

        let nodes = manager.get_healthy_nodes().await.unwrap();
        assert_eq!(nodes.len(), 0);
    }

    #[tokio::test]
    async fn test_get_all_nodes() {
        let manager = ClusterManager::new();
        manager.register_node("node1", "127.0.0.1:8000", 100).await.unwrap();
        manager.register_node("node2", "127.0.0.1:8001", 100).await.unwrap();

        let nodes = manager.get_all_nodes().await.unwrap();
        assert_eq!(nodes.len(), 2);
    }
}
