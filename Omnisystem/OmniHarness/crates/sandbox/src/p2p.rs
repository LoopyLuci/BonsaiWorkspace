//! P2P distribution system for mesh-based runtime sharing via TransferDaemon

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

/// Peer information for P2P mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub port: u16,
    pub bandwidth_mbps: u32,
    pub latency_ms: u32,
    pub available_runtimes: Vec<String>, // BLAKE3 hashes
}

/// P2P mesh statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeshStats {
    pub total_peers: usize,
    pub average_latency_ms: u32,
    pub total_bandwidth_mbps: u32,
    pub coverage: f64, // percentage of runtimes available on mesh
}

/// TransferDaemon P2P distribution manager
#[derive(Debug, Clone)]
pub struct P2PDistributor {
    peers: HashMap<String, PeerInfo>,
    #[allow(dead_code)]
    bootstrap_peers: Vec<String>,
    local_peer_id: String,
}

impl P2PDistributor {
    pub fn new(local_peer_id: String) -> Self {
        Self {
            peers: HashMap::new(),
            bootstrap_peers: vec![
                "bonsai-bootstrap-1.ecosystem".to_string(),
                "bonsai-bootstrap-2.ecosystem".to_string(),
                "bonsai-bootstrap-3.ecosystem".to_string(),
            ],
            local_peer_id,
        }
    }

    /// Register a peer in the mesh
    pub fn register_peer(&mut self, peer: PeerInfo) -> Result<()> {
        self.peers.insert(peer.peer_id.clone(), peer);
        Ok(())
    }

    /// Get list of peers that have a specific runtime
    pub fn find_peers_with_runtime(&self, hash: &str) -> Vec<&PeerInfo> {
        self.peers
            .values()
            .filter(|p| p.available_runtimes.contains(&hash.to_string()))
            .collect()
    }

    /// Select best peers for downloading (by latency and bandwidth)
    pub fn select_best_peers(&self, hash: &str, count: usize) -> Vec<&PeerInfo> {
        let mut peers = self.find_peers_with_runtime(hash);

        // Sort by combined score: lower latency + higher bandwidth = better
        peers.sort_by_key(|p| {
            let latency_score = p.latency_ms as i32;
            let bandwidth_score = -(p.bandwidth_mbps as i32);
            latency_score + bandwidth_score
        });

        peers.into_iter().take(count).collect()
    }

    /// Get mesh statistics
    pub fn get_stats(&self) -> MeshStats {
        if self.peers.is_empty() {
            return MeshStats::default();
        }

        let total_latency: u32 = self.peers.values().map(|p| p.latency_ms).sum();
        let average_latency = total_latency / self.peers.len() as u32;

        let total_bandwidth: u32 = self.peers.values().map(|p| p.bandwidth_mbps).sum();

        // Calculate runtime coverage (percentage of unique runtimes available)
        let mut all_runtimes = std::collections::HashSet::new();
        for peer in self.peers.values() {
            for runtime in &peer.available_runtimes {
                all_runtimes.insert(runtime.clone());
            }
        }

        MeshStats {
            total_peers: self.peers.len(),
            average_latency_ms: average_latency,
            total_bandwidth_mbps: total_bandwidth,
            coverage: if all_runtimes.is_empty() {
                0.0
            } else {
                (all_runtimes.len() as f64 / 100.0).min(1.0)
            },
        }
    }

    /// Simulate downloading from multiple peers (multi-path bonding)
    pub async fn download_from_best_peers(
        &self,
        hash: &str,
        size_mb: u64,
    ) -> Result<u64> {
        let peers = self.select_best_peers(hash, 3); // Use up to 3 peers

        if peers.is_empty() {
            return Err(anyhow::anyhow!("No peers available for runtime: {}", hash));
        }

        // Calculate combined bandwidth
        let total_bandwidth: u32 = peers.iter().map(|p| p.bandwidth_mbps).sum();

        if total_bandwidth == 0 {
            return Err(anyhow::anyhow!("No bandwidth available"));
        }

        // Estimate download time (in milliseconds)
        let size_bits = size_mb * 1024 * 1024 * 8;
        let total_bandwidth_bits_per_sec = total_bandwidth as u64 * 1024 * 1024;
        let download_time_sec = (size_bits as f64 / total_bandwidth_bits_per_sec as f64).ceil() as u64;

        // Add average latency for initial connection
        let avg_latency = peers.iter().map(|p| p.latency_ms).sum::<u32>() / peers.len() as u32;
        let total_time_ms = (download_time_sec * 1000) + avg_latency as u64;

        Ok(total_time_ms)
    }

    /// Announce local runtimes to the mesh
    pub fn announce_local_runtimes(&self, runtimes: Vec<String>) -> Result<()> {
        // In production: send announcement to bootstrap peers
        tracing::info!(
            "Announcing {} runtimes from peer {}",
            runtimes.len(),
            self.local_peer_id
        );
        Ok(())
    }

    /// Get all peers
    pub fn get_peers(&self) -> Vec<&PeerInfo> {
        self.peers.values().collect()
    }

    /// Clear all peers (for testing)
    pub fn clear(&mut self) {
        self.peers.clear();
    }
}

impl Default for P2PDistributor {
    fn default() -> Self {
        Self::new("local-peer".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2p_creation() {
        let p2p = P2PDistributor::new("test-peer".to_string());
        assert_eq!(p2p.local_peer_id, "test-peer");
        assert_eq!(p2p.bootstrap_peers.len(), 3);
    }

    #[test]
    fn test_register_peer() {
        let mut p2p = P2PDistributor::default();
        let peer = PeerInfo {
            peer_id: "peer-1".to_string(),
            address: "192.168.1.1".to_string(),
            port: 8080,
            bandwidth_mbps: 100,
            latency_ms: 10,
            available_runtimes: vec!["hash1".to_string(), "hash2".to_string()],
        };

        p2p.register_peer(peer).unwrap();
        assert_eq!(p2p.peers.len(), 1);
    }

    #[test]
    fn test_find_peers_with_runtime() {
        let mut p2p = P2PDistributor::default();
        p2p.register_peer(PeerInfo {
            peer_id: "peer-1".to_string(),
            address: "192.168.1.1".to_string(),
            port: 8080,
            bandwidth_mbps: 100,
            latency_ms: 10,
            available_runtimes: vec!["hash1".to_string()],
        })
        .ok();

        let peers = p2p.find_peers_with_runtime("hash1");
        assert_eq!(peers.len(), 1);

        let peers = p2p.find_peers_with_runtime("nonexistent");
        assert_eq!(peers.len(), 0);
    }

    #[test]
    fn test_mesh_stats() {
        let mut p2p = P2PDistributor::default();
        p2p.register_peer(PeerInfo {
            peer_id: "peer-1".to_string(),
            address: "192.168.1.1".to_string(),
            port: 8080,
            bandwidth_mbps: 100,
            latency_ms: 10,
            available_runtimes: vec!["hash1".to_string()],
        })
        .ok();

        let stats = p2p.get_stats();
        assert_eq!(stats.total_peers, 1);
        assert!(stats.coverage > 0.0);
    }

    #[tokio::test]
    async fn test_download_simulation() {
        let mut p2p = P2PDistributor::default();
        p2p.register_peer(PeerInfo {
            peer_id: "peer-1".to_string(),
            address: "192.168.1.1".to_string(),
            port: 8080,
            bandwidth_mbps: 100,
            latency_ms: 10,
            available_runtimes: vec!["hash1".to_string()],
        })
        .ok();

        let time_ms = p2p.download_from_best_peers("hash1", 30).await.unwrap();
        assert!(time_ms > 0);
    }
}
