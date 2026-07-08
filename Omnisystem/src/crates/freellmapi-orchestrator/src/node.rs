use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Healthy,
    Unhealthy,
    Draining,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub address: String,
    pub capacity: u32,
    pub current_load: u32,
    pub status: NodeStatus,
    pub created_at: u64,
    pub last_heartbeat: u64,
}

impl Node {
    pub fn new(id: String, address: String, capacity: u32) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Node {
            id,
            address,
            capacity,
            current_load: 0,
            status: NodeStatus::Healthy,
            created_at: now,
            last_heartbeat: now,
        }
    }

    pub fn available_capacity(&self) -> u32 {
        self.capacity.saturating_sub(self.current_load)
    }

    pub fn is_healthy(&self) -> bool {
        self.status == NodeStatus::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new("node1".to_string(), "127.0.0.1:8000".to_string(), 100);
        assert_eq!(node.id, "node1");
        assert_eq!(node.capacity, 100);
        assert_eq!(node.available_capacity(), 100);
        assert!(node.is_healthy());
    }

    #[test]
    fn test_node_load() {
        let mut node = Node::new("node1".to_string(), "127.0.0.1:8000".to_string(), 100);
        node.current_load = 60;

        assert_eq!(node.available_capacity(), 40);
    }

    #[test]
    fn test_node_status() {
        let mut node = Node::new("node1".to_string(), "127.0.0.1:8000".to_string(), 100);
        assert!(node.is_healthy());

        node.status = NodeStatus::Unhealthy;
        assert!(!node.is_healthy());
    }
}
