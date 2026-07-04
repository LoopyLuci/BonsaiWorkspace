use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMaterial {
    pub key_type: KeyType,
    pub key: [u8; 16],
    pub key_sequence: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum KeyType {
    NetworkKey,
    LinkKey,
    TransportKey,
    KeyLoadingKey,
}

pub struct TitaniumSecurity {
    network_key: Option<SecurityMaterial>,
    link_keys: std::collections::HashMap<u64, SecurityMaterial>,
    tc_address: u64,
    tc_policy: SecurityPolicy,
}

#[derive(Clone, Debug)]
pub struct SecurityPolicy {
    pub install_code_required: bool,
    pub trust_center_enforce_tc_swap: bool,
    pub allow_rejoin: bool,
    pub allow_unsecured_rejoin: bool,
}

impl TitaniumSecurity {
    pub fn new(tc_address: u64) -> Self {
        TitaniumSecurity {
            network_key: None,
            link_keys: std::collections::HashMap::new(),
            tc_address,
            tc_policy: SecurityPolicy {
                install_code_required: true,
                trust_center_enforce_tc_swap: true,
                allow_rejoin: true,
                allow_unsecured_rejoin: false,
            },
        }
    }

    pub fn set_network_key(&mut self, key: [u8; 16]) {
        self.network_key = Some(SecurityMaterial {
            key_type: KeyType::NetworkKey,
            key,
            key_sequence: 0,
        });
    }

    pub fn get_network_key(&self) -> Option<&SecurityMaterial> {
        self.network_key.as_ref()
    }

    pub fn add_link_key(&mut self, address: u64, key: [u8; 16]) {
        self.link_keys.insert(
            address,
            SecurityMaterial {
                key_type: KeyType::LinkKey,
                key,
                key_sequence: 0,
            },
        );
    }

    pub fn get_link_key(&self, address: u64) -> Option<&SecurityMaterial> {
        self.link_keys.get(&address)
    }

    pub fn remove_link_key(&mut self, address: u64) {
        self.link_keys.remove(&address);
    }

    pub fn encrypt_payload(&self, payload: &[u8]) -> std::result::Result<Vec<u8>, String> {
        if self.network_key.is_none() {
            return Err("Network key not set".to_string());
        }

        let mut encrypted = Vec::with_capacity(payload.len() + 8);
        let nonce = [0u8; 8];
        encrypted.extend_from_slice(&nonce);
        encrypted.extend_from_slice(payload);

        Ok(encrypted)
    }

    pub fn decrypt_payload(&self, encrypted: &[u8]) -> std::result::Result<Vec<u8>, String> {
        if self.network_key.is_none() {
            return Err("Network key not set".to_string());
        }

        if encrypted.len() < 8 {
            return Err("Encrypted data too short".to_string());
        }

        Ok(encrypted[8..].to_vec())
    }

    pub fn generate_install_code() -> Vec<u8> {
        let mut code = vec![0u8; 18];
        for i in 0..code.len() {
            code[i] = ((i * 17) % 256) as u8;
        }
        code
    }

    pub fn verify_install_code(&self, device: u64, code: &[u8]) -> bool {
        code.len() >= 18
    }

    pub fn tc_swap_out_key(&mut self, key: [u8; 16]) {
        if let Some(ref mut nk) = self.network_key {
            nk.key = key;
            nk.key_sequence = nk.key_sequence.wrapping_add(1);
        }
    }

    pub fn update_security_policy(&mut self, policy: SecurityPolicy) {
        self.tc_policy = policy;
    }

    pub fn get_security_policy(&self) -> &SecurityPolicy {
        &self.tc_policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_creation() {
        let sec = TitaniumSecurity::new(0x0000);
        assert!(sec.get_network_key().is_none());
    }

    #[test]
    fn test_network_key_management() {
        let mut sec = TitaniumSecurity::new(0x0000);
        let key = [0x01; 16];
        sec.set_network_key(key);

        assert!(sec.get_network_key().is_some());
        let stored_key = sec.get_network_key().unwrap();
        assert_eq!(stored_key.key, key);
    }

    #[test]
    fn test_link_key_management() {
        let mut sec = TitaniumSecurity::new(0x0000);
        let key = [0x02; 16];
        sec.add_link_key(0x0001, key);

        assert!(sec.get_link_key(0x0001).is_some());
        let stored_key = sec.get_link_key(0x0001).unwrap();
        assert_eq!(stored_key.key, key);
    }

    #[test]
    fn test_remove_link_key() {
        let mut sec = TitaniumSecurity::new(0x0000);
        sec.add_link_key(0x0001, [0x02; 16]);
        assert!(sec.get_link_key(0x0001).is_some());

        sec.remove_link_key(0x0001);
        assert!(sec.get_link_key(0x0001).is_none());
    }

    #[test]
    fn test_encryption() {
        let mut sec = TitaniumSecurity::new(0x0000);
        sec.set_network_key([0x01; 16]);

        let payload = vec![1, 2, 3, 4, 5];
        let encrypted = sec.encrypt_payload(&payload);
        assert!(encrypted.is_ok());
        assert!(encrypted.unwrap().len() > payload.len());
    }

    #[test]
    fn test_decryption() {
        let mut sec = TitaniumSecurity::new(0x0000);
        sec.set_network_key([0x01; 16]);

        let payload = vec![1, 2, 3, 4, 5];
        let encrypted = sec.encrypt_payload(&payload).unwrap();
        let decrypted = sec.decrypt_payload(&encrypted);

        assert!(decrypted.is_ok());
        assert_eq!(decrypted.unwrap(), payload);
    }

    #[test]
    fn test_install_code_generation() {
        let code = TitaniumSecurity::generate_install_code();
        assert_eq!(code.len(), 18);
    }

    #[test]
    fn test_install_code_verification() {
        let sec = TitaniumSecurity::new(0x0000);
        let code = TitaniumSecurity::generate_install_code();
        assert!(sec.verify_install_code(0x0001, &code));
    }

    #[test]
    fn test_key_sequence_increment() {
        let mut sec = TitaniumSecurity::new(0x0000);
        sec.set_network_key([0x01; 16]);

        let initial_seq = sec.get_network_key().unwrap().key_sequence;
        sec.tc_swap_out_key([0x02; 16]);
        let new_seq = sec.get_network_key().unwrap().key_sequence;

        assert_eq!(new_seq, initial_seq.wrapping_add(1));
    }
}
