// Cluster Management - Nodes, consensus, health tracking, failover

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Cluster Node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    pub node_id: String,
    pub address: String,
    pub port: u16,
    pub healthy: bool,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub role: String,
}

/// Node Status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub uptime_seconds: u64,
    pub request_count: u64,
    pub error_count: u32,
}

/// Cluster Manager
pub struct ClusterManager {
    cluster_id: String,
    node_id: String,
    nodes: Arc<DashMap<String, ClusterNode>>,
    node_statuses: Arc<DashMap<String, NodeStatus>>,
    is_leader: bool,
}

impl ClusterManager {
    pub async fn new(node_id: &str, peers: Vec<String>) -> anyhow::Result<Self> {
        tracing::info!("Initializing Cluster Manager for node: {}", node_id);

        let nodes = Arc::new(DashMap::new());

        // Add current node
        nodes.insert(
            node_id.to_string(),
            ClusterNode {
                node_id: node_id.to_string(),
                address: "127.0.0.1".to_string(),
                port: 9000,
                healthy: true,
                last_heartbeat: chrono::Utc::now(),
                role: "follower".to_string(),
            },
        );

        // Add peer nodes
        for (idx, peer) in peers.iter().enumerate() {
            nodes.insert(
                format!("peer-{}", idx),
                ClusterNode {
                    node_id: format!("peer-{}", idx),
                    address: peer.clone(),
                    port: 9000 + idx as u16,
                    healthy: true,
                    last_heartbeat: chrono::Utc::now(),
                    role: "follower".to_string(),
                },
            );
        }

        Ok(Self {
            cluster_id: uuid::Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            nodes,
            node_statuses: Arc::new(DashMap::new()),
            is_leader: false,
        })
    }

    pub async fn get_nodes(&self) -> anyhow::Result<Vec<ClusterNode>> {
        Ok(self.nodes.iter().map(|entry| entry.value().clone()).collect())
    }

    pub async fn get_node(&self, node_id: &str) -> anyhow::Result<ClusterNode> {
        self.nodes
            .get(node_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", node_id))
    }

    pub async fn add_node(&self, node: ClusterNode) -> anyhow::Result<()> {
        tracing::info!("Adding node to cluster: {}", node.node_id);
        self.nodes.insert(node.node_id.clone(), node);
        Ok(())
    }

    pub async fn remove_node(&self, node_id: &str) -> anyhow::Result<()> {
        tracing::info!("Removing node from cluster: {}", node_id);
        self.nodes.remove(node_id);
        Ok(())
    }

    pub async fn update_node_status(&self, status: NodeStatus) -> anyhow::Result<()> {
        tracing::debug!("Updating status for node: {}", status.node_id);
        self.node_statuses.insert(status.node_id.clone(), status);
        Ok(())
    }

    pub async fn get_node_status(&self, node_id: &str) -> anyhow::Result<NodeStatus> {
        self.node_statuses
            .get(node_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| anyhow::anyhow!("No status for node: {}", node_id))
    }

    pub async fn heartbeat(&self, node_id: &str) -> anyhow::Result<()> {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.last_heartbeat = chrono::Utc::now();
            node.healthy = true;
        }
        Ok(())
    }

    pub async fn check_node_health(&self) -> anyhow::Result<()> {
        let now = chrono::Utc::now();

        for mut entry in self.nodes.iter_mut() {
            let elapsed = (now - entry.value().last_heartbeat).num_seconds();
            if elapsed > 30 {
                entry.value_mut().healthy = false;
                tracing::warn!("Node {} marked unhealthy (no heartbeat for {} seconds)", entry.key(), elapsed);
            }
        }

        Ok(())
    }

    pub async fn elect_leader(&mut self) -> anyhow::Result<String> {
        tracing::info!("Initiating leader election");

        let healthy_nodes: Vec<_> = self
            .nodes
            .iter()
            .filter(|entry| entry.value().healthy)
            .map(|entry| entry.key().clone())
            .collect();

        if healthy_nodes.is_empty() {
            return Err(anyhow::anyhow!("No healthy nodes for leader election"));
        }

        let leader_id = healthy_nodes.first().unwrap().clone();
        self.is_leader = leader_id == self.node_id;

        if let Some(mut leader) = self.nodes.get_mut(&leader_id) {
            leader.role = "leader".to_string();
        }

        tracing::info!("Leader elected: {}", leader_id);
        Ok(leader_id)
    }

    pub async fn get_healthy_node_count(&self) -> anyhow::Result<usize> {
        Ok(self
            .nodes
            .iter()
            .filter(|entry| entry.value().healthy)
            .count())
    }

    pub async fn is_quorum_available(&self) -> anyhow::Result<bool> {
        let total_nodes = self.nodes.len();
        let healthy_nodes = self.get_healthy_node_count().await?;
        Ok(healthy_nodes > total_nodes / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_manager_creation() {
        let manager = ClusterManager::new("node-1", vec![]).await.unwrap();
        let nodes = manager.get_nodes().await.unwrap();
        assert!(!nodes.is_empty());
    }

    #[tokio::test]
    async fn test_add_node() {
        let manager = ClusterManager::new("node-1", vec![]).await.unwrap();
        let node = ClusterNode {
            node_id: "node-2".to_string(),
            address: "127.0.0.2".to_string(),
            port: 9001,
            healthy: true,
            last_heartbeat: chrono::Utc::now(),
            role: "follower".to_string(),
        };
        manager.add_node(node).await.unwrap();
        let nodes = manager.get_nodes().await.unwrap();
        assert_eq!(nodes.len(), 2);
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let manager = ClusterManager::new("node-1", vec![]).await.unwrap();
        manager.heartbeat("node-1").await.unwrap();
        let node = manager.get_node("node-1").await.unwrap();
        assert!(node.healthy);
    }

    #[tokio::test]
    async fn test_quorum_check() {
        let manager = ClusterManager::new("node-1", vec!["127.0.0.2".to_string()]).await.unwrap();
        let quorum = manager.is_quorum_available().await.unwrap();
        assert!(quorum);
    }
}
