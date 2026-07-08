//! Control Plane (Peer Discovery & Management)

use parking_lot::Mutex;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

pub struct PeerAdvertisement {
    pub node_id: Vec<u8>,
    pub endpoints: Vec<SocketAddr>,
    pub timestamp: u64,
}

pub struct ControlPlane {
    peers: Arc<Mutex<HashMap<Vec<u8>, PeerAdvertisement>>>,
}

impl ControlPlane {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_peer(&self, advert: PeerAdvertisement) {
        self.peers.lock().insert(advert.node_id.clone(), advert);
    }

    pub fn get_peer(&self, node_id: &[u8]) -> Option<PeerAdvertisement> {
        self.peers.lock().get(node_id).cloned()
    }

    pub fn list_peers(&self) -> Vec<PeerAdvertisement> {
        self.peers.lock().values().cloned().collect()
    }

    pub fn unregister_peer(&self, node_id: &[u8]) {
        self.peers.lock().remove(node_id);
    }
}

#[derive(Clone)]
pub struct PeerAdvertisement {
    pub node_id: Vec<u8>,
    pub endpoints: Vec<SocketAddr>,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_plane() {
        let cp = ControlPlane::new();
        let addr: SocketAddr = "1.2.3.4:51820".parse().unwrap();
        let advert = PeerAdvertisement {
            node_id: vec![1u8; 32],
            endpoints: vec![addr],
            timestamp: 0,
        };
        cp.register_peer(advert);
        assert_eq!(cp.list_peers().len(), 1);
    }
}
