//! WireGuard Cryptographic Operations
//!
//! Handles key generation, encryption, and decryption using Noise protocol.

use sha2::{Sha256, Digest};

#[derive(Clone)]
pub struct CryptoKey {
    pub bytes: Vec<u8>,
}

impl CryptoKey {
    pub fn new(bytes: Vec<u8>) -> Self {
        if bytes.len() != 32 {
            panic!("CryptoKey must be 32 bytes");
        }
        Self { bytes }
    }

    pub fn from_random() -> Self {
        // Stub: in production, use cryptographically secure RNG
        Self {
            bytes: vec![0xABu8; 32],
        }
    }

    pub fn public_key(&self) -> Vec<u8> {
        // Stub: in production, compute actual X25519 public key
        let mut hasher = Sha256::new();
        hasher.update(&self.bytes);
        hasher.finalize().to_vec()
    }

    pub fn shared_secret(&self, peer_public: &[u8]) -> Vec<u8> {
        // Stub: in production, use X25519 ECDH
        let mut hasher = Sha256::new();
        hasher.update(&self.bytes);
        hasher.update(peer_public);
        hasher.finalize().to_vec()
    }
}

pub struct CryptoOps {
    private_key: CryptoKey,
    session_key: Option<Vec<u8>>,
}

impl CryptoOps {
    pub fn new(private_key: Vec<u8>) -> Self {
        Self {
            private_key: CryptoKey::new(private_key),
            session_key: None,
        }
    }

    pub fn set_session_key(&mut self, key: Vec<u8>) {
        self.session_key = Some(key);
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let session_key = self.session_key.as_ref()
            .ok_or_else(|| "No session key established".to_string())?;

        // Stub: in production, use proper AEAD cipher (ChaCha20Poly1305)
        let mut ciphertext = Vec::with_capacity(plaintext.len() + 16);
        for (i, byte) in plaintext.iter().enumerate() {
            ciphertext.push(byte ^ session_key[i % session_key.len()]);
        }
        // Stub: append authentication tag
        ciphertext.extend_from_slice(&[0u8; 16]);

        Ok(ciphertext)
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        if ciphertext.len() < 16 {
            return Err("Ciphertext too short".to_string());
        }

        let session_key = self.session_key.as_ref()
            .ok_or_else(|| "No session key established".to_string())?;

        let encrypted = &ciphertext[..ciphertext.len() - 16];
        let mut plaintext = Vec::with_capacity(encrypted.len());
        for (i, byte) in encrypted.iter().enumerate() {
            plaintext.push(byte ^ session_key[i % session_key.len()]);
        }

        // Stub: verify authentication tag
        Ok(plaintext)
    }

    pub fn public_key(&self) -> Vec<u8> {
        self.private_key.public_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_key_creation() {
        let key = CryptoKey::new(vec![0u8; 32]);
        assert_eq!(key.bytes.len(), 32);
    }

    #[test]
    fn test_crypto_key_public() {
        let key = CryptoKey::new(vec![42u8; 32]);
        let public = key.public_key();
        assert_eq!(public.len(), 32);
    }

    #[test]
    fn test_encryption_decryption() {
        let mut ops = CryptoOps::new(vec![1u8; 32]);
        ops.set_session_key(vec![2u8; 32]);

        let plaintext = b"Hello, WireGuard!";
        let ciphertext = ops.encrypt(plaintext).unwrap();
        assert!(ciphertext.len() > plaintext.len());

        let decrypted = ops.decrypt(&ciphertext).unwrap();
        assert_eq!(&decrypted[..], plaintext);
    }
}
