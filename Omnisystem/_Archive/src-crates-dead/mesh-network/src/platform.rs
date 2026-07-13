//! Mesh Network Platform Integration
//!
//! Complete platform combining coordination, routing, DNS, and relay services.
//! This is the user-facing API for the mesh network.

use crate::coordination::{MeshNode, NetworkState, ACLRule, ACLAction, MeshStats};
use crate::mesh_routing::PacketRouter;
use crate::dns::MagicDNS;
use crate::relay::RelayNetwork;
use std::net::IpAddr;
use std::sync::Arc;

/// Platform configuration
#[derive(Clone)]
pub struct MeshConfig {
    pub network_name: String,
    pub heartbeat_interval_secs: u64,
    pub offline_timeout_secs: u64,
    pub relay_preferred_region: Option<String>,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            network_name: "default-tailnet".to_string(),
            heartbeat_interval_secs: 30,
            offline_timeout_secs: 300,
            relay_preferred_region: None,
        }
    }
}

/// Complete mesh network platform
pub struct MeshPlatform {
    config: MeshConfig,
    state: Arc<NetworkState>,
    router: Arc<PacketRouter>,
    dns: Arc<MagicDNS>,
    relay_network: Arc<RelayNetwork>,
}

impl MeshPlatform {
    pub fn new(config: MeshConfig) -> Self {
        let network_id = config.network_name.as_bytes().to_vec();
        let state = Arc::new(NetworkState::new(network_id));
        let router = Arc::new(PacketRouter::new(state.clone()));
        let dns = Arc::new(MagicDNS::new(state.clone()));
        let relay_network = Arc::new(RelayNetwork::new());

        Self {
            config,
            state,
            router,
            dns,
            relay_network,
        }
    }

    // --- Node Management ---

    pub fn register_node(&self, node: MeshNode) -> Result<(), String> {
        self.state.register_node(node)?;
        self.router.compute_routes();
        self.dns.sync_from_mesh();
        Ok(())
    }

    pub fn get_node(&self, node_id: &[u8]) -> Option<MeshNode> {
        self.state.get_node(node_id)
    }

    pub fn list_nodes(&self) -> Vec<MeshNode> {
        self.state.list_nodes()
    }

    pub fn heartbeat(&self, node_id: &[u8]) -> Result<(), String> {
        self.state.heartbeat(node_id)
    }

    pub fn cleanup_offline_nodes(&self) -> usize {
        self.state.cleanup_offline_nodes(self.config.offline_timeout_secs)
    }

    // --- Access Control ---

    pub fn add_acl_rule(&self, rule: ACLRule) {
        self.state.add_acl_rule(rule);
    }

    pub fn remove_acl_rule(&self, rule_id: &str) {
        self.state.remove_acl_rule(rule_id);
    }

    pub fn allow_traffic(
        &self,
        from_node: Vec<u8>,
        to_node: Vec<u8>,
    ) -> Result<(), String> {
        self.state.check_acl(&from_node, &to_node, &["TCP", "UDP"])?;
        Ok(())
    }

    pub fn deny_traffic(&self, rule_id: String, from_node: Vec<u8>, to_node: Vec<u8>) {
        let rule = ACLRule {
            id: rule_id,
            source_node: from_node,
            dest_node: to_node,
            action: ACLAction::Deny,
            ports: None,
            protocols: vec!["TCP".to_string(), "UDP".to_string()],
            priority: 1000, // High priority deny
        };
        self.add_acl_rule(rule);
    }

    // --- Routing ---

    pub fn compute_routes(&self) {
        self.router.compute_routes();
    }

    pub fn get_peer_list(&self, node_id: &[u8]) -> Result<Vec<MeshNode>, String> {
        self.state.get_peer_list(node_id)
    }

    pub fn route_exists(&self, src: IpAddr, dest: IpAddr) -> bool {
        let decision = self.router.decide_forwarding(src, dest);
        decision.should_forward
    }

    // --- DNS ---

    pub fn resolve_name(&self, domain: &str) -> Option<IpAddr> {
        self.dns.resolve_with_fallthrough(domain)
    }

    pub fn reverse_lookup(&self, ip: IpAddr) -> Option<String> {
        self.dns.reverse_lookup(ip)
    }

    pub fn list_dns_names(&self) -> Vec<String> {
        self.dns.list_names()
    }

    // --- Relay/NAT Traversal ---

    pub fn establish_relay_connection(
        &self,
        local_node: Vec<u8>,
        remote_node: Vec<u8>,
    ) -> Result<String, String> {
        let conn = self.relay_network.establish_relay(
            local_node,
            remote_node,
            self.config.relay_preferred_region.as_deref(),
        )?;

        Ok(format!(
            "{}-{}",
            hex::encode(&conn.local_node[..4]),
            hex::encode(&conn.remote_node[..4])
        ))
    }

    pub fn close_relay_connection(&self, conn_id: &str) -> Result<(), String> {
        self.relay_network.close_relay(conn_id)
    }

    pub fn list_relay_servers(&self) -> Vec<String> {
        self.relay_network
            .list_servers()
            .iter()
            .map(|s| format!("{} ({})", s.server_id, s.region))
            .collect()
    }

    // --- Metrics & Status ---

    pub fn get_network_stats(&self) -> PlatformStats {
        let mesh_stats = self.state.get_stats();
        let relay_stats = self.relay_network.get_stats();

        PlatformStats {
            total_nodes: mesh_stats.total_nodes,
            online_nodes: mesh_stats.online_nodes,
            packets_processed: mesh_stats.packets_processed,
            acl_rules: mesh_stats.acl_rules,
            active_relays: relay_stats.active_connections,
            relay_packets: relay_stats.packets_relayed,
            relay_bytes: relay_stats.bytes_relayed,
        }
    }

    pub fn get_node_peers(&self, node_id: &[u8]) -> Result<Vec<String>, String> {
        let peers = self.state.get_peer_list(node_id)?;
        Ok(peers
            .iter()
            .map(|p| format!("{} ({})", p.name, hex::encode(&p.node_id[..4])))
            .collect())
    }

    pub fn network_health(&self) -> NetworkHealth {
        let stats = self.get_network_stats();
        let total = stats.total_nodes.max(1);
        let online_ratio = stats.online_nodes as f64 / total as f64;

        NetworkHealth {
            is_healthy: online_ratio > 0.8,
            online_ratio,
            network_utilization: (stats.relay_packets as f64) / 1_000_000.0, // Normalized
            acl_violations: 0, // Placeholder
        }
    }
}

pub struct PlatformStats {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub packets_processed: u64,
    pub acl_rules: usize,
    pub active_relays: usize,
    pub relay_packets: u64,
    pub relay_bytes: u64,
}

pub struct NetworkHealth {
    pub is_healthy: bool,
    pub online_ratio: f64,
    pub network_utilization: f64,
    pub acl_violations: u32,
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_creation() {
        let config = MeshConfig::default();
        let platform = MeshPlatform::new(config);
        let stats = platform.get_network_stats();
        assert_eq!(stats.total_nodes, 0);
    }

    #[test]
    fn test_node_registration() {
        let platform = MeshPlatform::new(MeshConfig::default());
        let node = MeshNode::new(vec![1u8; 32], "device1".to_string());
        assert!(platform.register_node(node).is_ok());
        assert_eq!(platform.get_network_stats().total_nodes, 1);
    }

    #[test]
    fn test_routing() {
        let platform = MeshPlatform::new(MeshConfig::default());
        let src: IpAddr = "10.0.0.1".parse().unwrap();
        let dest: IpAddr = "10.0.0.2".parse().unwrap();

        platform.compute_routes();
        // No route without nodes
        assert!(!platform.route_exists(src, dest));
    }

    #[test]
    fn test_acl_management() {
        let platform = MeshPlatform::new(MeshConfig::default());
        let rule = ACLRule {
            id: "rule1".to_string(),
            source_node: vec![1u8; 32],
            dest_node: vec![2u8; 32],
            action: ACLAction::Allow,
            ports: None,
            protocols: vec!["TCP".to_string()],
            priority: 100,
        };
        platform.add_acl_rule(rule);
        assert_eq!(platform.get_network_stats().acl_rules, 1);
    }

    #[test]
    fn test_network_health() {
        let platform = MeshPlatform::new(MeshConfig::default());
        let health = platform.network_health();
        assert!(!health.is_healthy); // No nodes = unhealthy
    }
}
