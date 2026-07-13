//! WireGuard Peer Management

use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    Down,
    Up,
    Reconnecting,
}

#[derive(Clone)]
pub struct Peer {
    pub public_key: Vec<u8>,
    pub preshared_key: Option<Vec<u8>>,
    pub endpoint: Option<SocketAddr>,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: Option<u16>,
    pub state: PeerState,
}

impl Peer {
    pub fn new(public_key: Vec<u8>) -> Self {
        Self {
            public_key,
            preshared_key: None,
            endpoint: None,
            allowed_ips: Vec::new(),
            persistent_keepalive: None,
            state: PeerState::Down,
        }
    }

    pub fn with_endpoint(mut self, endpoint: SocketAddr) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub fn with_allowed_ips(mut self, ips: Vec<String>) -> Self {
        self.allowed_ips = ips;
        self
    }

    pub fn with_keepalive(mut self, seconds: u16) -> Self {
        self.persistent_keepalive = Some(seconds);
        self
    }

    pub fn with_preshared_key(mut self, key: Vec<u8>) -> Self {
        self.preshared_key = Some(key);
        self
    }

    pub fn set_state(&mut self, state: PeerState) {
        self.state = state;
    }

    /// Get last handshake timestamp (stub)
    pub fn last_handshake(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Get bytes received (stub - should track actual stats)
    pub fn bytes_received(&self) -> u64 {
        0
    }

    /// Get bytes sent (stub - should track actual stats)
    pub fn bytes_sent(&self) -> u64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_creation() {
        let peer = Peer::new(vec![1u8; 32]);
        assert_eq!(peer.public_key.len(), 32);
        assert_eq!(peer.state, PeerState::Down);
    }

    #[test]
    fn test_peer_builder() {
        let peer = Peer::new(vec![1u8; 32])
            .with_keepalive(25)
            .with_allowed_ips(vec!["10.0.0.1/32".to_string()]);

        assert_eq!(peer.persistent_keepalive, Some(25));
        assert_eq!(peer.allowed_ips.len(), 1);
    }
}
