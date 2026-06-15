//! Distributed Cache Clustering
//!
//! Consistent hashing with replication and failure detection.

use dashmap::DashMap;
use parking_lot::Mutex;
use std::collections::{HashMap, BTreeSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Cluster node
#[derive(Clone, Debug)]
pub struct ClusterNode {
    pub node_id: String,
    pub address: SocketAddr,
    pub is_alive: Arc<AtomicU64>,
    pub replication_factor: u32,
}

impl ClusterNode {
    pub fn new(node_id: String, address: SocketAddr) -> Self {
        Self {
            node_id,
            address,
            is_alive: Arc::new(AtomicU64::new(1)),
            replication_factor: 3,
        }
    }

    pub fn mark_alive(&self) {
        self.is_alive.store(1, Ordering::Relaxed);
    }

    pub fn mark_dead(&self) {
        self.is_alive.store(0, Ordering::Relaxed);
    }

    pub fn is_healthy(&self) -> bool {
        self.is_alive.load(Ordering::Relaxed) == 1
    }
}

/// Consistent hash ring
pub struct HashRing {
    ring: BTreeSet<u64>,
    node_map: HashMap<u64, ClusterNode>,
    virtual_nodes: u32,
}

impl HashRing {
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            ring: BTreeSet::new(),
            node_map: HashMap::new(),
            virtual_nodes,
        }
    }

    pub fn add_node(&mut self, node: ClusterNode) {
        for i in 0..self.virtual_nodes {
            let hash = Self::hash(&format!("{}:{}", node.node_id, i));
            self.ring.insert(hash);
            self.node_map.insert(hash, node.clone());
        }
    }

    pub fn remove_node(&mut self, node_id: &str) {
        let to_remove: Vec<_> = self
            .node_map
            .iter()
            .filter(|(_, n)| n.node_id == node_id)
            .map(|(h, _)| *h)
            .collect();

        for hash in to_remove {
            self.ring.remove(&hash);
            self.node_map.remove(&hash);
        }
    }

    pub fn get_responsible_nodes(&self, key: &str, replication: u32) -> Vec<ClusterNode> {
        let hash = Self::hash(key);
        let mut nodes = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let mut iter = self.ring.range(hash..);
        if iter.next().is_none() {
            iter = self.ring.range(..);
        }

        for &hash_val in iter.chain(self.ring.iter()) {
            if let Some(node) = self.node_map.get(&hash_val) {
                if !seen.contains(&node.node_id) && node.is_healthy() {
                    nodes.push(node.clone());
                    seen.insert(node.node_id.clone());

                    if nodes.len() >= replication as usize {
                        break;
                    }
                }
            }
        }

        nodes
    }

    fn hash(key: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    pub fn node_count(&self) -> usize {
        self.node_map
            .values()
            .map(|n| n.node_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
}

/// Cluster manager
pub struct ClusterManager {
    ring: Arc<Mutex<HashRing>>,
    nodes: Arc<DashMap<String, ClusterNode>>,
    replication_factor: u32,
}

impl ClusterManager {
    pub fn new(replication_factor: u32) -> Self {
        Self {
            ring: Arc::new(Mutex::new(HashRing::new(150))),
            nodes: Arc::new(DashMap::new()),
            replication_factor,
        }
    }

    pub fn join(&self, node: ClusterNode) {
        self.nodes.insert(node.node_id.clone(), node.clone());
        self.ring.lock().add_node(node);
    }

    pub fn leave(&self, node_id: &str) {
        self.nodes.remove(node_id);
        self.ring.lock().remove_node(node_id);
    }

    pub fn get_replicas(&self, key: &str) -> Vec<ClusterNode> {
        self.ring.lock().get_responsible_nodes(key, self.replication_factor)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_node(&self, node_id: &str) -> Option<ClusterNode> {
        self.nodes.get(node_id).map(|n| n.clone())
    }

    pub fn list_nodes(&self) -> Vec<ClusterNode> {
        self.nodes
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn detect_failures(&self) -> Vec<String> {
        let failed: Vec<String> = self
            .nodes
            .iter()
            .filter(|n| !n.value().is_healthy())
            .map(|n| n.key().clone())
            .collect();

        for node_id in failed.iter() {
            self.leave(node_id);
        }

        failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_ring_creation() {
        let ring = HashRing::new(150);
        assert_eq!(ring.node_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut ring = HashRing::new(150);
        let addr: SocketAddr = "127.0.0.1:6379".parse().unwrap();
        let node = ClusterNode::new("node1".to_string(), addr);
        ring.add_node(node);
        assert_eq!(ring.node_count(), 1);
    }

    #[test]
    fn test_consistent_hashing() {
        let mut ring = HashRing::new(150);
        let addr1: SocketAddr = "127.0.0.1:6379".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:6380".parse().unwrap();

        ring.add_node(ClusterNode::new("node1".to_string(), addr1));
        ring.add_node(ClusterNode::new("node2".to_string(), addr2));

        let nodes = ring.get_responsible_nodes("test-key", 2);
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_cluster_manager() {
        let manager = ClusterManager::new(3);
        let addr: SocketAddr = "127.0.0.1:6379".parse().unwrap();
        let node = ClusterNode::new("node1".to_string(), addr);
        manager.join(node);
        assert_eq!(manager.node_count(), 1);
    }

    #[test]
    fn test_replication() {
        let manager = ClusterManager::new(2);
        let addr1: SocketAddr = "127.0.0.1:6379".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:6380".parse().unwrap();

        manager.join(ClusterNode::new("node1".to_string(), addr1));
        manager.join(ClusterNode::new("node2".to_string(), addr2));

        let replicas = manager.get_replicas("key");
        assert!(replicas.len() <= 2);
    }
}
