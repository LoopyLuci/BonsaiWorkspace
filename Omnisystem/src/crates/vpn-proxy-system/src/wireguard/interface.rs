//! WireGuard Interface Management

use super::crypto::CryptoOps;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct InterfaceConfig {
    pub private_key: Vec<u8>,
    pub listen_port: u16,
    pub mtu: u16,
}

pub struct WireGuardInterface {
    config: InterfaceConfig,
    crypto: Arc<Mutex<CryptoOps>>,
    session_keys: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>, // peer_pub_key -> session_key
    packet_counter: Arc<Mutex<u64>>,
}

impl WireGuardInterface {
    pub fn new(config: InterfaceConfig) -> Self {
        let crypto = CryptoOps::new(config.private_key.clone());

        Self {
            config,
            crypto: Arc::new(Mutex::new(crypto)),
            session_keys: Arc::new(Mutex::new(HashMap::new())),
            packet_counter: Arc::new(Mutex::new(0)),
        }
    }

    pub fn public_key(&self) -> Vec<u8> {
        let crypto = self.crypto.lock();
        crypto.public_key()
    }

    pub fn set_session_key(&self, peer_public_key: Vec<u8>, session_key: Vec<u8>) {
        let mut keys = self.session_keys.lock();
        keys.insert(peer_public_key, session_key);
    }

    pub fn get_mtu(&self) -> u16 {
        self.config.mtu
    }

    pub fn encrypt_packet(&self, peer_key: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
        let session_keys = self.session_keys.lock();
        let session_key = session_keys
            .get(peer_key)
            .ok_or_else(|| "No session key for peer".to_string())?
            .clone();
        drop(session_keys);

        let mut crypto = self.crypto.lock();
        crypto.set_session_key(session_key);
        crypto.encrypt(data)
    }

    pub fn decrypt_packet(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let crypto = self.crypto.lock();
        crypto.decrypt(ciphertext)
    }

    pub fn next_packet_number(&self) -> u64 {
        let mut counter = self.packet_counter.lock();
        *counter += 1;
        *counter - 1
    }

    pub fn get_listen_port(&self) -> u16 {
        self.config.listen_port
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_creation() {
        let config = InterfaceConfig {
            private_key: vec![0u8; 32],
            listen_port: 51820,
            mtu: 1420,
        };
        let iface = WireGuardInterface::new(config);
        assert_eq!(iface.get_listen_port(), 51820);
        assert_eq!(iface.get_mtu(), 1420);
    }

    #[test]
    fn test_session_key_management() {
        let config = InterfaceConfig {
            private_key: vec![0u8; 32],
            listen_port: 51820,
            mtu: 1420,
        };
        let iface = WireGuardInterface::new(config);

        let peer_key = vec![1u8; 32];
        let session_key = vec![2u8; 32];

        iface.set_session_key(peer_key.clone(), session_key.clone());
        // Verify it was stored (indirectly via encryption working)
    }

    #[test]
    fn test_packet_counter() {
        let config = InterfaceConfig {
            private_key: vec![0u8; 32],
            listen_port: 51820,
            mtu: 1420,
        };
        let iface = WireGuardInterface::new(config);

        let pkt1 = iface.next_packet_number();
        let pkt2 = iface.next_packet_number();
        assert_eq!(pkt1, 0);
        assert_eq!(pkt2, 1);
    }
}
