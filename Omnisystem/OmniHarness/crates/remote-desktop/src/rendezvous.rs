//! Peer discovery and registration using mDNS and hole punching.
//!
//! The RendezvousService coordinates peer discovery through multicast DNS (mDNS),
//! registration in a central registry, and NAT hole punching for direct peer connections.

use crate::core::Core;
use crate::PeerId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use chrono::{DateTime, Utc};

/// Errors that can occur during peer discovery.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("Peer not found: {peer_id}")]
    PeerNotFound { peer_id: String },

    #[error("Registration failed: {reason}")]
    RegistrationFailed { reason: String },

    #[error("NAT hole punching failed: {reason}")]
    HolePunchingFailed { reason: String },

    #[error("mDNS query failed: {reason}")]
    QueryFailed { reason: String },

    #[error("Invalid peer address")]
    InvalidAddress,

    #[error("Discovery timeout")]
    Timeout,
}

/// Information about a discovered peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier.
    pub id: PeerId,

    /// Human-readable peer name.
    pub name: String,

    /// Network address(es) of the peer.
    pub addresses: Vec<SocketAddr>,

    /// Whether the peer is behind NAT.
    pub behind_nat: bool,

    /// Last time we heard from this peer.
    pub last_seen: DateTime<Utc>,

    /// Peer capabilities (e.g., "capture", "input").
    pub capabilities: Vec<String>,

    /// Whether this peer is currently online.
    pub online: bool,

    /// Public key for initial handshake (if available).
    pub public_key: Option<Vec<u8>>,
}

impl PeerInfo {
    /// Create a new PeerInfo.
    pub fn new(id: PeerId, name: String) -> Self {
        PeerInfo {
            id,
            name,
            addresses: vec![],
            behind_nat: false,
            last_seen: Utc::now(),
            capabilities: vec![],
            online: true,
            public_key: None,
        }
    }

    /// Add an address to this peer's address list.
    pub fn with_address(mut self, addr: SocketAddr) -> Self {
        self.addresses.push(addr);
        self
    }

    /// Mark this peer as behind NAT.
    pub fn with_nat(mut self) -> Self {
        self.behind_nat = true;
        self
    }

    /// Add a capability to this peer.
    pub fn with_capability(mut self, capability: String) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Set the public key for handshake.
    pub fn with_public_key(mut self, key: Vec<u8>) -> Self {
        self.public_key = Some(key);
        self
    }
}

/// Service for peer discovery and registration.
pub struct RendezvousService {
    /// Registered peers (PeerId -> PeerInfo).
    peers: Arc<DashMap<PeerId, PeerInfo>>,

    /// mDNS listener active flag.
    mdns_active: Arc<std::sync::atomic::AtomicBool>,

    /// Cache of resolved hole-punch addresses, keyed by hex-encoded PeerId.
    hole_punch_cache: Core,
}

impl RendezvousService {
    /// Create a new RendezvousService.
    pub fn new() -> Self {
        RendezvousService {
            peers: Arc::new(DashMap::new()),
            mdns_active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            hole_punch_cache: Core::new(),
        }
    }

    /// Start the rendezvous service (mDNS listener, etc.).
    pub async fn start(&self) -> Result<(), DiscoveryError> {
        if self.mdns_active.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        // In production, this would spawn:
        // 1. mDNS listener (via mdns-sd or similar)
        // 2. UDP hole punching responder
        // 3. Periodic cleanup of stale peers

        tracing::info!("RendezvousService started");
        Ok(())
    }

    /// Register a peer in the discovery service.
    pub async fn register_peer(&self, peer_info: PeerInfo) -> Result<(), DiscoveryError> {
        if peer_info.addresses.is_empty() {
            return Err(DiscoveryError::RegistrationFailed {
                reason: "No addresses provided".to_string(),
            });
        }

        self.peers.insert(peer_info.id, peer_info);
        Ok(())
    }

    /// Discover peers on the local network via mDNS.
    pub async fn discover_peers(&self) -> Result<Vec<PeerInfo>, DiscoveryError> {
        // In production, this would:
        // 1. Query mDNS for _bonsai-rd._tcp.local
        // 2. Perform hole punching if behind NAT
        // 3. Filter and return reachable peers

        let mut result = vec![];
        for entry in self.peers.iter() {
            let peer = entry.value().clone();
            if peer.online {
                result.push(peer);
            }
        }

        if result.is_empty() {
            return Err(DiscoveryError::QueryFailed {
                reason: "No peers found".to_string(),
            });
        }

        Ok(result)
    }

    /// Find a specific peer by ID.
    pub async fn find_peer(&self, peer_id: PeerId) -> Result<PeerInfo, DiscoveryError> {
        self.peers
            .get(&peer_id)
            .map(|entry| entry.value().clone())
            .ok_or(DiscoveryError::PeerNotFound {
                peer_id: peer_id.to_string(),
            })
    }

    /// Perform NAT hole punching for a peer. Resolved addresses are cached
    /// per peer so repeat calls skip re-resolution.
    pub async fn hole_punch(&self, peer_id: PeerId) -> Result<SocketAddr, DiscoveryError> {
        let cache_key = peer_id.to_string();
        if let Some(cached) = self.hole_punch_cache.get(&cache_key) {
            if let Ok(addr) = cached.parse() {
                return Ok(addr);
            }
        }

        let peer = self.find_peer(peer_id).await?;

        let addr = if !peer.behind_nat {
            // No hole punching needed
            peer.addresses.first().cloned()
                .ok_or(DiscoveryError::InvalidAddress)?
        } else {
            // In production: perform STUN-based hole punching
            // For now: return the first address
            peer.addresses
                .first()
                .cloned()
                .ok_or(DiscoveryError::HolePunchingFailed {
                    reason: "No addresses available".to_string(),
                })?
        };

        self.hole_punch_cache.set(cache_key, addr.to_string());
        Ok(addr)
    }

    /// Update peer info (last seen, online status, etc.).
    pub async fn update_peer(&self, peer_id: PeerId, info: PeerInfo) -> Result<(), DiscoveryError> {
        self.hole_punch_cache.remove(&peer_id.to_string());
        self.peers.insert(peer_id, info);
        Ok(())
    }

    /// Mark a peer as offline.
    pub async fn mark_offline(&self, peer_id: PeerId) -> Result<(), DiscoveryError> {
        self.hole_punch_cache.remove(&peer_id.to_string());
        if let Some(mut entry) = self.peers.get_mut(&peer_id) {
            entry.online = false;
            Ok(())
        } else {
            Err(DiscoveryError::PeerNotFound {
                peer_id: peer_id.to_string(),
            })
        }
    }

    /// Get the current number of registered peers.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Clear all peers (for testing).
    #[cfg(test)]
    pub fn clear(&self) {
        self.peers.clear();
    }
}

impl Default for RendezvousService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_register_and_discover() {
        let service = RendezvousService::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);
        let addr = "127.0.0.1:5000".parse().unwrap();

        let peer_info = PeerInfo::new(peer_id, "test-peer".to_string())
            .with_address(addr);

        service.register_peer(peer_info).await.unwrap();

        let discovered = service.discover_peers().await.unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].id, peer_id);
    }

    #[tokio::test]
    async fn test_find_peer() {
        let service = RendezvousService::new();
        let peer_id = PeerId::from_bytes(&[2u8; 32]);
        let addr = "127.0.0.1:5001".parse().unwrap();

        let peer_info = PeerInfo::new(peer_id, "peer-2".to_string())
            .with_address(addr);

        service.register_peer(peer_info).await.unwrap();
        let found = service.find_peer(peer_id).await.unwrap();
        assert_eq!(found.id, peer_id);
    }

    #[tokio::test]
    async fn test_peer_not_found() {
        let service = RendezvousService::new();
        let unknown_id = PeerId::from_bytes(&[99u8; 32]);

        let result = service.find_peer(unknown_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mark_offline() {
        let service = RendezvousService::new();
        let peer_id = PeerId::from_bytes(&[3u8; 32]);
        let addr = "127.0.0.1:5002".parse().unwrap();

        let peer_info = PeerInfo::new(peer_id, "peer-3".to_string())
            .with_address(addr);

        service.register_peer(peer_info).await.unwrap();
        service.mark_offline(peer_id).await.unwrap();

        let discovered = service.discover_peers().await;
        assert!(discovered.is_err() || discovered.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_nat_peer() {
        let service = RendezvousService::new();
        let peer_id = PeerId::from_bytes(&[4u8; 32]);
        let addr = "192.168.1.1:5003".parse().unwrap();

        let peer_info = PeerInfo::new(peer_id, "nat-peer".to_string())
            .with_address(addr)
            .with_nat();

        assert!(peer_info.behind_nat);
        service.register_peer(peer_info).await.unwrap();
        let found = service.find_peer(peer_id).await.unwrap();
        assert!(found.behind_nat);
    }

    #[test]
    fn test_peer_count() {
        let service = RendezvousService::new();
        let peer_id = PeerId::from_bytes(&[5u8; 32]);
        let addr = "127.0.0.1:5004".parse().unwrap();

        let peer_info = PeerInfo::new(peer_id, "peer-5".to_string())
            .with_address(addr);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            service.register_peer(peer_info).await.unwrap();
        });

        assert_eq!(service.peer_count(), 1);
    }
}
