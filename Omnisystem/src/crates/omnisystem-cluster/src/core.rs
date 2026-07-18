//! Cluster manager facade tying membership, consensus, and status
//! reporting together for a single node.

use crate::membership::MembershipManager;
use crate::Result;

/// Snapshot of a node's view of the cluster.
#[derive(Debug, Clone)]
pub struct ClusterStatus {
    pub node_id: String,
    pub is_leader: bool,
    pub term: u64,
}

/// Facade for a single cluster node: owns a unique id and a handle to
/// cluster membership.
pub struct ClusterManager {
    node_id: String,
    membership: MembershipManager,
}

impl ClusterManager {
    /// Create a new cluster node with a fresh id and empty membership view.
    pub async fn new() -> Result<Self> {
        let node_id = uuid::Uuid::new_v4().to_string();
        let membership = MembershipManager::new().await?;
        Ok(Self { node_id, membership })
    }

    /// This node's unique id.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Cheap handle to this node's membership manager (shares state via Arc).
    pub fn membership(&self) -> MembershipManager {
        self.membership.clone()
    }

    /// Current status snapshot for this node.
    pub async fn get_status(&self) -> Result<ClusterStatus> {
        Ok(ClusterStatus {
            node_id: self.node_id.clone(),
            is_leader: false,
            term: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_manager_new() {
        let mgr = ClusterManager::new().await.unwrap();
        assert!(!mgr.node_id().is_empty());
    }

    #[tokio::test]
    async fn test_cluster_manager_status() {
        let mgr = ClusterManager::new().await.unwrap();
        let status = mgr.get_status().await.unwrap();
        assert_eq!(status.node_id, mgr.node_id());
        assert!(!status.is_leader);
        assert_eq!(status.term, 0);
    }

    #[tokio::test]
    async fn test_cluster_manager_membership() {
        let mgr = ClusterManager::new().await.unwrap();
        mgr.membership().add_node("node-x").await.unwrap();
        let nodes = mgr.membership().get_nodes().await.unwrap();
        assert_eq!(nodes, vec!["node-x".to_string()]);
    }
}
