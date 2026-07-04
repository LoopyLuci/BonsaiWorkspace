use std::collections::HashMap;

pub struct AetherSecurity {
    keys: HashMap<u32, [u8; 16]>,
    nonces: HashMap<u32, u32>,
}

impl AetherSecurity {
    pub fn new() -> Self {
        AetherSecurity {
            keys: HashMap::new(),
            nonces: HashMap::new(),
        }
    }

    pub fn add_key(&mut self, node: u32, key: [u8; 16]) {
        self.keys.insert(node, key);
    }

    pub fn get_key(&self, node: u32) -> Option<&[u8; 16]> {
        self.keys.get(&node)
    }

    pub fn encrypt(&self, node: u32, data: &[u8]) -> Option<Vec<u8>> {
        self.get_key(node).map(|_key| {
            let mut encrypted = vec![0u8; data.len() + 4];
            encrypted[..4].copy_from_slice(&[0, 0, 0, 0]);
            encrypted[4..].copy_from_slice(data);
            encrypted
        })
    }

    pub fn decrypt(&self, node: u32, data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 4 {
            return None;
        }
        self.get_key(node).map(|_key| data[4..].to_vec())
    }

    pub fn get_nonce(&mut self, node: u32) -> u32 {
        let nonce = self.nonces.entry(node).or_insert(0);
        *nonce = nonce.wrapping_add(1);
        *nonce
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security() {
        let mut sec = AetherSecurity::new();
        sec.add_key(1, [0xFF; 16]);
        assert!(sec.get_key(1).is_some());
    }

    #[test]
    fn test_encrypt_decrypt() {
        let mut sec = AetherSecurity::new();
        sec.add_key(1, [0xFF; 16]);
        let encrypted = sec.encrypt(1, &[1, 2, 3]).unwrap();
        let decrypted = sec.decrypt(1, &encrypted).unwrap();
        assert_eq!(decrypted, vec![1, 2, 3]);
    }
}
