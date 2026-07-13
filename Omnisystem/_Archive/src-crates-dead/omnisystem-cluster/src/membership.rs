/// Membership Management Module

use crate::Result;
use tracing::info;

/// Membership manager
pub struct MembershipManager {
    nodes: std::sync::Arc<tokio::sync::RwLock<Vec<String>>>,
}

impl MembershipManager {
    pub async fn new() -> Result<Self> {
        info!("Initializing Membership Manager");
        Ok(Self {
            nodes: std::sync::Arc::new(tokio::sync::RwLock::new(vec![])),
        })
    }

    pub async fn add_node(&self, node_id: &str) -> Result<()> {
        info!("Adding node to cluster: {}", node_id);
        let mut nodes = self.nodes.write().await;
        nodes.push(node_id.to_string());
        Ok(())
    }

    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        info!("Removing node from cluster: {}", node_id);
        let mut nodes = self.nodes.write().await;
        nodes.retain(|n| n != node_id);
        Ok(())
    }

    pub async fn get_nodes(&self) -> Result<Vec<String>> {
        let nodes = self.nodes.read().await;
        Ok(nodes.clone())
    }
}
