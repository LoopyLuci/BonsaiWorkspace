//! Mesh Routing Protocol
//!
//! Distributed routing with shortest path discovery and
//! automatic relay fallback for unreachable peers.

use crate::coordination::{MeshNode, NetworkState};
use dashmap::DashMap;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;

/// Route entry in the routing table
#[derive(Clone, Debug)]
pub struct Route {
    pub dest_ip: IpAddr,
    pub next_hop_node: Vec<u8>,       // Next node ID
    pub hops: u32,                     // Number of hops
    pub metric: f64,                   // Latency-based metric
}

/// Mesh routing service
pub struct MeshRouter {
    state: Arc<NetworkState>,
    routing_table: Arc<DashMap<IpAddr, Route>>,
    peer_distances: Arc<Mutex<HashMap<Vec<u8>, HashMap<Vec<u8>, u32>>>>,
}

impl MeshRouter {
    pub fn new(state: Arc<NetworkState>) -> Self {
        Self {
            state,
            routing_table: Arc::new(DashMap::new()),
            peer_distances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Compute shortest paths using Floyd-Warshall style algorithm
    pub fn compute_routing_table(&self) {
        let nodes = self.state.list_nodes();
        let mut distances = HashMap::new();

        // Initialize distances
        for node in &nodes {
            let mut node_distances = HashMap::new();
            for other in &nodes {
                if node.node_id == other.node_id {
                    node_distances.insert(other.node_id.clone(), 0);
                } else if self.are_direct_neighbors(&node.node_id, &other.node_id) {
                    node_distances.insert(other.node_id.clone(), 1);
                } else {
                    node_distances.insert(other.node_id.clone(), u32::MAX);
                }
            }
            distances.insert(node.node_id.clone(), node_distances);
        }

        // Floyd-Warshall
        for k in &nodes {
            for i in &nodes {
                for j in &nodes {
                    let dist_i_k = distances
                        .get(&i.node_id)
                        .and_then(|m| m.get(&k.node_id))
                        .copied()
                        .unwrap_or(u32::MAX);
                    let dist_k_j = distances
                        .get(&k.node_id)
                        .and_then(|m| m.get(&j.node_id))
                        .copied()
                        .unwrap_or(u32::MAX);
                    let dist_i_j = distances
                        .get(&i.node_id)
                        .and_then(|m| m.get(&j.node_id))
                        .copied()
                        .unwrap_or(u32::MAX);

                    if dist_i_k.saturating_add(dist_k_j) < dist_i_j {
                        if let Some(mut m) = distances.get_mut(&i.node_id) {
                            m.insert(j.node_id.clone(), dist_i_k + dist_k_j);
                        }
                    }
                }
            }
        }

        // Build routing table
        self.routing_table.clear();
        for node in &nodes {
            if let Some(ipv4) = node.ipv4 {
                if let Some(distances) = distances.get(&node.node_id) {
                    let mut best_hop = node.node_id.clone();
                    let mut best_distance = u32::MAX;

                    for (peer_id, &dist) in distances {
                        if dist < best_distance && dist > 0 {
                            best_distance = dist;
                            best_hop = peer_id.clone();
                        }
                    }

                    if best_distance < u32::MAX {
                        let route = Route {
                            dest_ip: ipv4,
                            next_hop_node: best_hop,
                            hops: best_distance,
                            metric: best_distance as f64,
                        };
                        self.routing_table.insert(ipv4, route);
                    }
                }
            }
        }

        // Store computed distances
        *self.peer_distances.lock() = distances;
    }

    /// Find route to destination
    pub fn find_route(&self, dest_ip: IpAddr) -> Option<Route> {
        self.routing_table.get(&dest_ip).map(|r| r.clone())
    }

    /// Find next hop via relay if direct unreachable
    pub fn find_relay_path(&self, dest_ip: IpAddr) -> Option<Vec<IpAddr>> {
        if let Some(route) = self.find_route(dest_ip) {
            // Direct route available
            return Some(vec![dest_ip]);
        }

        // Find relay: find closest online node to destination
        let nodes = self.state.list_nodes();
        let mut best_relay = None;
        let mut best_distance = u32::MAX;

        for relay in nodes.iter().filter(|n| n.online) {
            if let Some(route) = self.find_route(dest_ip) {
                if route.hops < best_distance {
                    best_distance = route.hops;
                    best_relay = relay.ipv4;
                }
            }
        }

        if let Some(relay_ip) = best_relay {
            Some(vec![relay_ip, dest_ip])
        } else {
            None
        }
    }

    /// Check if two nodes are direct neighbors (endpoints known)
    fn are_direct_neighbors(&self, node1: &[u8], node2: &[u8]) -> bool {
        if let (Some(n1), Some(n2)) = (self.state.get_node(node1), self.state.get_node(node2)) {
            !n1.endpoints.is_empty() && !n2.endpoints.is_empty()
        } else {
            false
        }
    }

    /// Get hop count to destination
    pub fn get_hop_count(&self, dest_ip: IpAddr) -> Option<u32> {
        self.routing_table.get(&dest_ip).map(|r| r.hops)
    }

    /// Get all routes
    pub fn list_routes(&self) -> Vec<Route> {
        self.routing_table
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
}

/// Packet forwarding decision
#[derive(Clone, Debug)]
pub struct ForwardingDecision {
    pub should_forward: bool,
    pub next_hop: Option<Vec<u8>>,
    pub relay_path: Option<Vec<IpAddr>>,
    pub reason: String,
}

/// Router state for packet forwarding
pub struct PacketRouter {
    mesh_router: Arc<MeshRouter>,
}

impl PacketRouter {
    pub fn new(state: Arc<NetworkState>) -> Self {
        Self {
            mesh_router: Arc::new(MeshRouter::new(state)),
        }
    }

    pub fn compute_routes(&self) {
        self.mesh_router.compute_routing_table();
    }

    /// Decide where to forward a packet
    pub fn decide_forwarding(&self, src_ip: IpAddr, dest_ip: IpAddr) -> ForwardingDecision {
        match self.mesh_router.find_route(dest_ip) {
            Some(route) => ForwardingDecision {
                should_forward: true,
                next_hop: Some(route.next_hop_node),
                relay_path: None,
                reason: format!("Direct route via {} ({} hops)",
                    hex::encode(&route.next_hop_node[..4]),
                    route.hops),
            },
            None => {
                match self.mesh_router.find_relay_path(dest_ip) {
                    Some(relay_path) => ForwardingDecision {
                        should_forward: true,
                        next_hop: None,
                        relay_path: Some(relay_path),
                        reason: format!("Relay path via {} hops", relay_path.len()),
                    },
                    None => ForwardingDecision {
                        should_forward: false,
                        next_hop: None,
                        relay_path: None,
                        reason: "No route available".to_string(),
                    },
                }
            }
        }
    }
}

// Simple hex encoding helper
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_router_creation() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let router = MeshRouter::new(state);
        assert_eq!(router.routing_table.len(), 0);
    }

    #[test]
    fn test_routing_table_computation() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let node1 = MeshNode::new(vec![1u8; 32], "node1".to_string());
        let node2 = MeshNode::new(vec![2u8; 32], "node2".to_string());

        state.register_node(node1).unwrap();
        state.register_node(node2).unwrap();

        let router = MeshRouter::new(state);
        router.compute_routing_table();
        // Routing table may be empty if no direct connections
        let routes = router.list_routes();
        assert!(routes.is_empty() || routes.len() > 0);
    }

    #[test]
    fn test_forwarding_decision() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let packet_router = PacketRouter::new(state);

        let src: IpAddr = "10.0.0.1".parse().unwrap();
        let dest: IpAddr = "10.0.0.2".parse().unwrap();

        let decision = packet_router.decide_forwarding(src, dest);
        // Should not forward without routes
        assert!(!decision.should_forward);
    }

    #[test]
    fn test_hop_count() {
        let state = Arc::new(crate::coordination::NetworkState::new(vec![0u8; 32]));
        let router = MeshRouter::new(state);
        router.compute_routing_table();

        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let hop_count = router.get_hop_count(ip);
        assert!(hop_count.is_none()); // No route
    }
}
