//! WireGuard Protocol Implementation
//!
//! Fast, modern VPN protocol with minimal attack surface.
//! Fully integrated with TransferDaemon for identity management,
//! post-quantum cryptography, and zero-trust peer authentication.

pub mod peer;
pub mod crypto;
pub mod interface;
pub mod packet;
pub mod transfer_daemon_integration;

pub use peer::{Peer, PeerState};
pub use crypto::{CryptoKey, CryptoOps};
pub use interface::{WireGuardInterface, InterfaceConfig};
pub use packet::{Message, MessageType};
pub use transfer_daemon_integration::{
    SelfCertifyingIdentity, HybridCryptoKey, TDPeer, WireGuardTD, WireGuardTDMetrics,
};

use parking_lot::Mutex;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

/// WireGuard tunnel instance
pub struct WireGuard {
    interface: Arc<Mutex<WireGuardInterface>>,
    peers: Arc<Mutex<HashMap<Vec<u8>, Peer>>>,
}

impl WireGuard {
    /// Create new WireGuard instance
    pub fn new(config: InterfaceConfig) -> Self {
        Self {
            interface: Arc::new(Mutex::new(WireGuardInterface::new(config))),
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a new peer
    pub fn add_peer(&self, peer: Peer) -> Result<(), String> {
        let mut peers = self.peers.lock();
        peers.insert(peer.public_key.clone(), peer);
        Ok(())
    }

    /// Remove peer by public key
    pub fn remove_peer(&self, public_key: &[u8]) -> Result<(), String> {
        let mut peers = self.peers.lock();
        peers.remove(public_key);
        Ok(())
    }

    /// Get peer by public key
    pub fn get_peer(&self, public_key: &[u8]) -> Option<Peer> {
        let peers = self.peers.lock();
        peers.get(public_key).cloned()
    }

    /// Process incoming packet
    pub fn process_packet(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let interface = self.interface.lock();
        interface.decrypt_packet(data)
    }

    /// Encrypt outgoing packet for peer
    pub fn encrypt_for_peer(&self, peer_key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
        let interface = self.interface.lock();
        interface.encrypt_packet(peer_key, data)
    }

    /// Get all peer public keys
    pub fn list_peers(&self) -> Vec<Vec<u8>> {
        let peers = self.peers.lock();
        peers.keys().cloned().collect()
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        let peers = self.peers.lock();
        peers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wireguard_creation() {
        let config = InterfaceConfig {
            private_key: vec![0u8; 32],
            listen_port: 51820,
            mtu: 1420,
        };
        let wg = WireGuard::new(config);
        assert_eq!(wg.peer_count(), 0);
    }

    #[test]
    fn test_peer_management() {
        let config = InterfaceConfig {
            private_key: vec![0u8; 32],
            listen_port: 51820,
            mtu: 1420,
        };
        let wg = WireGuard::new(config);

        let peer = Peer {
            public_key: vec![1u8; 32],
            preshared_key: None,
            endpoint: None,
            allowed_ips: vec!["10.0.0.1/32".to_string()],
            persistent_keepalive: Some(25),
            state: PeerState::Down,
        };

        assert!(wg.add_peer(peer).is_ok());
        assert_eq!(wg.peer_count(), 1);
    }
}
