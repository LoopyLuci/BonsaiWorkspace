//! Control Plane Coordination Service
//!
//! Central coordination service that maintains mesh topology,
//! distributes peer information, and manages network state.

use dashmap::DashMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Node in the mesh network
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshNode {
    pub node_id: Vec<u8>,               // Public key = identity
    pub name: String,                   // Human-readable name
    pub ipv4: Option<IpAddr>,          // Assigned IPv4 in tailnet
    pub ipv6: Option<IpAddr>,          // Assigned IPv6 in tailnet
    pub endpoints: Vec<SocketAddr>,    // Known endpoints (public IPs)
    pub region: String,                // Geographic region
    pub os: String,                    // Operating system
    pub hostname: String,              // Device hostname
    pub registered_at: u64,            // Registration timestamp
    pub last_seen: Arc<AtomicU64>,     // Last heartbeat
    pub online: bool,                  // Currently online
}

impl MeshNode {
    pub fn new(node_id: Vec<u8>, name: String) -> Self {
        Self {
            node_id,
            name,
            ipv4: None,
            ipv6: None,
            endpoints: Vec::new(),
            region: "unknown".to_string(),
            os: "unknown".to_string(),
            hostname: "unknown".to_string(),
            registered_at: Self::current_time(),
            last_seen: Arc::new(AtomicU64::new(Self::current_time())),
            online: false,
        }
    }

    pub fn update_endpoint(&mut self, addr: SocketAddr) {
        if !self.endpoints.contains(&addr) {
            self.endpoints.push(addr);
            self.endpoints.truncate(10); // Keep last 10 endpoints
        }
        self.last_seen.store(Self::current_time(), Ordering::Relaxed);
    }

    pub fn mark_online(&mut self) {
        self.online = true;
        self.last_seen.store(Self::current_time(), Ordering::Relaxed);
    }

    pub fn mark_offline(&mut self) {
        self.online = false;
    }

    pub fn is_stale(&self, timeout_secs: u64) -> bool {
        let last_seen = self.last_seen.load(Ordering::Relaxed);
        let now = Self::current_time();
        now.saturating_sub(last_seen) > timeout_secs
    }

    fn current_time() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Access Control Policy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ACLRule {
    pub id: String,
    pub source_node: Vec<u8>,          // Source node ID
    pub dest_node: Vec<u8>,            // Destination node ID
    pub action: ACLAction,             // Allow/Deny
    pub ports: Option<(u16, u16)>,     // Port range (min, max)
    pub protocols: Vec<String>,        // TCP, UDP, ICMP
    pub priority: u32,                 // Higher = evaluated first
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ACLAction {
    Allow,
    Deny,
}

/// Network state managed by coordination service
#[derive(Clone)]
pub struct NetworkState {
    pub network_id: Vec<u8>,                           // Network identifier
    pub nodes: Arc<DashMap<Vec<u8>, MeshNode>>,      // All nodes by ID
    pub acl_rules: Arc<DashMap<String, ACLRule>>,    // Access control
    pub dns_names: Arc<DashMap<String, Vec<u8>>>,    // DNS name -> node ID
    pub ipv4_pool: Arc<Mutex<Vec<IpAddr>>>,          // Available IPv4s
    pub ipv6_pool: Arc<Mutex<Vec<IpAddr>>>,          // Available IPv6s
    pub issued_ipv4s: Arc<DashMap<Vec<u8>, IpAddr>>, // Node -> IPv4
    pub issued_ipv6s: Arc<DashMap<Vec<u8>, IpAddr>>, // Node -> IPv6
    pub stats_packets: Arc<AtomicU64>,                // Packets processed
    pub stats_nodes: Arc<AtomicU64>,                  // Total nodes seen
}

impl NetworkState {
    pub fn new(network_id: Vec<u8>) -> Self {
        Self {
            network_id,
            nodes: Arc::new(DashMap::new()),
            acl_rules: Arc::new(DashMap::new()),
            dns_names: Arc::new(DashMap::new()),
            ipv4_pool: Arc::new(Mutex::new(Vec::new())),
            ipv6_pool: Arc::new(Mutex::new(Vec::new())),
            issued_ipv4s: Arc::new(DashMap::new()),
            issued_ipv6s: Arc::new(DashMap::new()),
            stats_packets: Arc::new(AtomicU64::new(0)),
            stats_nodes: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn register_node(&self, mut node: MeshNode) -> Result<(), String> {
        // Assign IPs
        if let Some(ipv4) = self.allocate_ipv4() {
            node.ipv4 = Some(ipv4);
            self.issued_ipv4s.insert(node.node_id.clone(), ipv4);
        }
        if let Some(ipv6) = self.allocate_ipv6() {
            node.ipv6 = Some(ipv6);
            self.issued_ipv6s.insert(node.node_id.clone(), ipv6);
        }

        self.nodes.insert(node.node_id.clone(), node.clone());
        self.dns_names.insert(node.name.clone(), node.node_id.clone());
        self.stats_nodes.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    fn allocate_ipv4(&self) -> Option<IpAddr> {
        let mut pool = self.ipv4_pool.lock();
        pool.pop()
    }

    fn allocate_ipv6(&self) -> Option<IpAddr> {
        let mut pool = self.ipv6_pool.lock();
        pool.pop()
    }

    pub fn get_node(&self, node_id: &[u8]) -> Option<MeshNode> {
        self.nodes.get(node_id).map(|n| n.clone())
    }

    pub fn list_nodes(&self) -> Vec<MeshNode> {
        self.nodes
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_peer_list(&self, node_id: &[u8]) -> Result<Vec<MeshNode>, String> {
        let requester = self
            .get_node(node_id)
            .ok_or_else(|| "Node not found".to_string())?;

        let mut peers = Vec::new();
        for entry in self.nodes.iter() {
            let peer = entry.value().clone();
            if peer.node_id != requester.node_id && peer.online {
                // Check ACL
                if self.check_acl(&requester.node_id, &peer.node_id, &["TCP", "UDP"])? {
                    peers.push(peer);
                }
            }
        }

        Ok(peers)
    }

    pub fn check_acl(
        &self,
        src: &[u8],
        dst: &[u8],
        protocols: &[&str],
    ) -> Result<bool, String> {
        let mut allowed = false;

        // Iterate ACL rules in priority order
        let mut rules: Vec<_> = self
            .acl_rules
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in rules {
            if rule.source_node == src && rule.dest_node == dst {
                match rule.action {
                    ACLAction::Allow => {
                        allowed = true;
                        break;
                    }
                    ACLAction::Deny => return Ok(false),
                }
            }
        }

        Ok(allowed)
    }

    pub fn add_acl_rule(&self, rule: ACLRule) {
        self.acl_rules.insert(rule.id.clone(), rule);
    }

    pub fn remove_acl_rule(&self, rule_id: &str) {
        self.acl_rules.remove(rule_id);
    }

    pub fn heartbeat(&self, node_id: &[u8]) -> Result<(), String> {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.last_seen.store(MeshNode::current_time(), Ordering::Relaxed);
            node.mark_online();
        }
        self.stats_packets.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn cleanup_offline_nodes(&self, timeout_secs: u64) -> usize {
        let mut removed = 0;
        let to_remove: Vec<_> = self
            .nodes
            .iter()
            .filter(|entry| entry.value().is_stale(timeout_secs))
            .map(|entry| entry.key().clone())
            .collect();

        for node_id in to_remove {
            self.nodes.remove(&node_id);
            self.issued_ipv4s.remove(&node_id);
            self.issued_ipv6s.remove(&node_id);
            removed += 1;
        }

        removed
    }

    pub fn get_stats(&self) -> MeshStats {
        MeshStats {
            total_nodes: self.nodes.len(),
            online_nodes: self.nodes.iter().filter(|n| n.value().online).count(),
            packets_processed: self.stats_packets.load(Ordering::Relaxed),
            acl_rules: self.acl_rules.len(),
        }
    }
}

pub struct MeshStats {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub packets_processed: u64,
    pub acl_rules: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_node_creation() {
        let node = MeshNode::new(vec![1u8; 32], "device1".to_string());
        assert_eq!(node.name, "device1");
        assert!(node.endpoints.is_empty());
    }

    #[test]
    fn test_network_state_register() {
        let state = NetworkState::new(vec![0u8; 32]);
        let node = MeshNode::new(vec![1u8; 32], "device1".to_string());
        assert!(state.register_node(node).is_ok());
        assert_eq!(state.nodes.len(), 1);
    }

    #[test]
    fn test_acl_rule() {
        let state = NetworkState::new(vec![0u8; 32]);
        let rule = ACLRule {
            id: "rule1".to_string(),
            source_node: vec![1u8; 32],
            dest_node: vec![2u8; 32],
            action: ACLAction::Allow,
            ports: Some((80, 443)),
            protocols: vec!["TCP".to_string()],
            priority: 100,
        };
        state.add_acl_rule(rule);
        assert_eq!(state.acl_rules.len(), 1);
    }

    #[test]
    fn test_node_online_status() {
        let mut node = MeshNode::new(vec![1u8; 32], "device1".to_string());
        assert!(!node.online);
        node.mark_online();
        assert!(node.online);
        node.mark_offline();
        assert!(!node.online);
    }

    #[test]
    fn test_mesh_stats() {
        let state = NetworkState::new(vec![0u8; 32]);
        let node1 = MeshNode::new(vec![1u8; 32], "device1".to_string());
        let node2 = MeshNode::new(vec![2u8; 32], "device2".to_string());

        state.register_node(node1).unwrap();
        state.register_node(node2).unwrap();

        let stats = state.get_stats();
        assert_eq!(stats.total_nodes, 2);
    }
}
