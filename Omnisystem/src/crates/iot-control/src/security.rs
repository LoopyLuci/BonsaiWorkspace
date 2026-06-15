use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct SecurityManager {
    psk_keys: Arc<DashMap<String, Vec<u8>>>,
    certificates: Arc<DashMap<String, String>>,
    tls_enabled: bool,
}

impl SecurityManager {
    pub fn new(tls_enabled: bool) -> Self {
        Self {
            psk_keys: Arc::new(DashMap::new()),
            certificates: Arc::new(DashMap::new()),
            tls_enabled,
        }
    }

    pub fn register_psk(&self, device_id: String, key: Vec<u8>) -> Result<()> {
        if key.len() < 16 {
            return Err(crate::IotError::ProtocolError("Key too short".to_string()));
        }
        self.psk_keys.insert(device_id, key);
        tracing::info!("PSK registered");
        Ok(())
    }

    pub fn verify_device(&self, device_id: &str) -> bool {
        self.psk_keys.contains_key(device_id) || self.certificates.contains_key(device_id)
    }

    pub fn encrypt_message(&self, device_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        if !self.verify_device(device_id) {
            return Err(crate::IotError::ProtocolError("Device not authorized".to_string()));
        }
        let mut encrypted = data.to_vec();
        for byte in &mut encrypted {
            *byte ^= 0xAA;
        }
        Ok(encrypted)
    }

    pub fn decrypt_message(&self, device_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        if !self.verify_device(device_id) {
            return Err(crate::IotError::ProtocolError("Device not authorized".to_string()));
        }
        let mut decrypted = data.to_vec();
        for byte in &mut decrypted {
            *byte ^= 0xAA;
        }
        Ok(decrypted)
    }

    pub fn device_count(&self) -> usize {
        self.psk_keys.len() + self.certificates.len()
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager() {
        let mgr = SecurityManager::new(true);
        assert!(mgr.register_psk("dev1".to_string(), vec![0; 16]).is_ok());
        assert!(mgr.verify_device("dev1"));
    }

    #[test]
    fn test_encryption() {
        let mgr = SecurityManager::new(true);
        mgr.register_psk("dev1".to_string(), vec![0; 16]).unwrap();
        let encrypted = mgr.encrypt_message("dev1", b"hello").unwrap();
        assert_ne!(encrypted, b"hello");
    }
}
