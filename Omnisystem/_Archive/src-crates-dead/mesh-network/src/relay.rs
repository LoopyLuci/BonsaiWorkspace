//! Relay Service (DERP-like)
//!
//! Relay servers for tunneling traffic when direct peer connection fails.
//! Handles NAT traversal fallback and geographic distribution.

use dashmap::DashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Relay server info
#[derive(Clone, Debug)]
pub struct RelayServer {
    pub server_id: String,
    pub region: String,
    pub address: SocketAddr,
    pub capacity: u32,
    pub current_connections: Arc<AtomicU64>,
}

impl RelayServer {
    pub fn new(server_id: String, region: String, address: SocketAddr) -> Self {
        Self {
            server_id,
            region,
            address,
            capacity: 10000,
            current_connections: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn can_accept_connection(&self) -> bool {
        self.current_connections.load(Ordering::Relaxed) < self.capacity as u64
    }

    pub fn add_connection(&self) {
        self.current_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn remove_connection(&self) {
        self.current_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn connection_count(&self) -> u64 {
        self.current_connections.load(Ordering::Relaxed)
    }

    pub fn utilization(&self) -> f64 {
        self.connection_count() as f64 / self.capacity as f64
    }
}

/// Relayed packet
#[derive(Clone, Debug)]
pub struct RelayedPacket {
    pub src_node: Vec<u8>,
    pub dest_node: Vec<u8>,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

/// Relay connection state
#[derive(Clone, Debug)]
pub struct RelayConnection {
    pub local_node: Vec<u8>,
    pub remote_node: Vec<u8>,
    pub relay_server: String,
    pub packets_relayed: Arc<AtomicU64>,
    pub bytes_relayed: Arc<AtomicU64>,
}

impl RelayConnection {
    pub fn new(local_node: Vec<u8>, remote_node: Vec<u8>, relay_server: String) -> Self {
        Self {
            local_node,
            remote_node,
            relay_server,
            packets_relayed: Arc::new(AtomicU64::new(0)),
            bytes_relayed: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_packet(&self, size: usize) {
        self.packets_relayed.fetch_add(1, Ordering::Relaxed);
        self.bytes_relayed.fetch_add(size as u64, Ordering::Relaxed);
    }
}

/// Relay network manager
pub struct RelayNetwork {
    servers: Arc<DashMap<String, RelayServer>>,
    connections: Arc<DashMap<String, RelayConnection>>,
    stats_packets: Arc<AtomicU64>,
    stats_bytes: Arc<AtomicU64>,
}

impl RelayNetwork {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(DashMap::new()),
            connections: Arc::new(DashMap::new()),
            stats_packets: Arc::new(AtomicU64::new(0)),
            stats_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Register relay server
    pub fn register_server(&self, server: RelayServer) {
        self.servers.insert(server.server_id.clone(), server);
    }

    /// Get relay server by ID
    pub fn get_server(&self, server_id: &str) -> Option<RelayServer> {
        self.servers.get(server_id).map(|s| s.clone())
    }

    /// Find best relay server by region and utilization
    pub fn find_best_relay(&self, preferred_region: Option<&str>) -> Option<RelayServer> {
        let mut candidates: Vec<_> = self
            .servers
            .iter()
            .filter(|s| s.value().can_accept_connection())
            .map(|s| s.value().clone())
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Prefer matching region
        if let Some(region) = preferred_region {
            if let Some(server) = candidates.iter().find(|s| s.region == region) {
                return Some(server.clone());
            }
        }

        // Otherwise pick least loaded
        candidates.sort_by(|a, b| {
            a.utilization()
                .partial_cmp(&b.utilization())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.first().cloned()
    }

    /// Establish relay connection
    pub fn establish_relay(
        &self,
        local_node: Vec<u8>,
        remote_node: Vec<u8>,
        preferred_region: Option<&str>,
    ) -> Result<RelayConnection, String> {
        let relay = self
            .find_best_relay(preferred_region)
            .ok_or_else(|| "No available relay servers".to_string())?;

        relay.add_connection();

        let conn = RelayConnection::new(local_node, remote_node, relay.server_id);
        let conn_id = format!(
            "{}-{}",
            hex::encode(&conn.local_node[..4]),
            hex::encode(&conn.remote_node[..4])
        );
        self.connections.insert(conn_id, conn.clone());

        Ok(conn)
    }

    /// Relay packet from source to destination
    pub fn relay_packet(
        &self,
        conn_id: &str,
        packet: RelayedPacket,
    ) -> Result<(), String> {
        if let Some(conn) = self.connections.get(conn_id) {
            conn.record_packet(packet.payload.len());
            self.stats_packets.fetch_add(1, Ordering::Relaxed);
            self.stats_bytes
                .fetch_add(packet.payload.len() as u64, Ordering::Relaxed);
            Ok(())
        } else {
            Err("Connection not found".to_string())
        }
    }

    /// Close relay connection
    pub fn close_relay(&self, conn_id: &str) -> Result<(), String> {
        if let Some((_, conn)) = self.connections.remove(conn_id) {
            if let Some(server) = self.servers.get(&conn.relay_server) {
                server.remove_connection();
            }
            Ok(())
        } else {
            Err("Connection not found".to_string())
        }
    }

    /// List active relay servers
    pub fn list_servers(&self) -> Vec<RelayServer> {
        self.servers
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List active connections
    pub fn list_connections(&self) -> Vec<RelayConnection> {
        self.connections
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get relay stats
    pub fn get_stats(&self) -> RelayStats {
        RelayStats {
            total_servers: self.servers.len(),
            active_connections: self.connections.len(),
            packets_relayed: self.stats_packets.load(Ordering::Relaxed),
            bytes_relayed: self.stats_bytes.load(Ordering::Relaxed),
            total_capacity: self
                .servers
                .iter()
                .map(|s| s.value().capacity as u64)
                .sum(),
        }
    }
}

impl Default for RelayNetwork {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RelayStats {
    pub total_servers: usize,
    pub active_connections: usize,
    pub packets_relayed: u64,
    pub bytes_relayed: u64,
    pub total_capacity: u64,
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
    use std::net::SocketAddr;

    #[test]
    fn test_relay_server_creation() {
        let addr: SocketAddr = "1.2.3.4:3478".parse().unwrap();
        let server = RelayServer::new("relay1".to_string(), "us-west".to_string(), addr);
        assert_eq!(server.server_id, "relay1");
        assert!(server.can_accept_connection());
    }

    #[test]
    fn test_relay_network() {
        let network = RelayNetwork::new();
        let addr: SocketAddr = "1.2.3.4:3478".parse().unwrap();
        let server = RelayServer::new("relay1".to_string(), "us-west".to_string(), addr);
        network.register_server(server);
        assert_eq!(network.list_servers().len(), 1);
    }

    #[test]
    fn test_relay_connection() {
        let network = RelayNetwork::new();
        let addr: SocketAddr = "1.2.3.4:3478".parse().unwrap();
        let server = RelayServer::new("relay1".to_string(), "us-west".to_string(), addr);
        network.register_server(server);

        let result = network.establish_relay(vec![1u8; 32], vec![2u8; 32], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_relay_packet() {
        let network = RelayNetwork::new();
        let addr: SocketAddr = "1.2.3.4:3478".parse().unwrap();
        let server = RelayServer::new("relay1".to_string(), "us-west".to_string(), addr);
        network.register_server(server);

        let conn = network
            .establish_relay(vec![1u8; 32], vec![2u8; 32], None)
            .unwrap();
        let conn_id = format!(
            "{}-{}",
            hex::encode(&conn.local_node[..4]),
            hex::encode(&conn.remote_node[..4])
        );

        let packet = RelayedPacket {
            src_node: vec![1u8; 32],
            dest_node: vec![2u8; 32],
            payload: b"hello".to_vec(),
            timestamp: 0,
        };

        assert!(network.relay_packet(&conn_id, packet).is_ok());
    }

    #[test]
    fn test_relay_stats() {
        let network = RelayNetwork::new();
        let stats = network.get_stats();
        assert_eq!(stats.total_servers, 0);
        assert_eq!(stats.active_connections, 0);
    }
}
