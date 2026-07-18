//! Real AES-256-GCM encryption at rest, plus lightweight in-memory key
//! management (key generation and rotation tracking).

use crate::{Result, SecurityError};
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use rand::RngCore;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

pub struct EncryptionManager;

impl EncryptionManager {
    pub fn new() -> Self {
        Self
    }

    /// Encrypt `data` with AES-256-GCM under `key` (must be exactly 32
    /// bytes). A random 12-byte nonce is generated per call and prepended to
    /// the returned ciphertext so `decrypt_at_rest` can recover it.
    pub fn encrypt_at_rest(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if key.len() != 32 {
            return Err(SecurityError::EncryptionError(format!(
                "key must be 32 bytes, got {}",
                key.len()
            )));
        }

        let cipher = Aes256Gcm::new(key.into());
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))?;

        let mut out = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypt data produced by `encrypt_at_rest`.
    pub fn decrypt_at_rest(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if key.len() != 32 {
            return Err(SecurityError::EncryptionError(format!(
                "key must be 32 bytes, got {}",
                key.len()
            )));
        }
        if data.len() < 12 {
            return Err(SecurityError::EncryptionError(
                "ciphertext too short to contain a nonce".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let cipher = Aes256Gcm::new(key.into());
        let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| SecurityError::EncryptionError(e.to_string()))
    }
}

impl Default for EncryptionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks generated keys and a rotation counter. Keys are held only
/// in-memory for the lifetime of the process.
pub struct KeyManager {
    generation: AtomicU32,
    current_key: Mutex<Vec<u8>>,
}

impl KeyManager {
    pub fn new() -> Self {
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        Self {
            generation: AtomicU32::new(1),
            current_key: Mutex::new(key),
        }
    }

    /// Generate a fresh random 256-bit key (does not affect `current_key`).
    pub fn generate_key(&self) -> Result<Vec<u8>> {
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        Ok(key)
    }

    /// Replace the current key with a freshly generated one and bump the
    /// generation counter.
    pub fn rotate_key(&self) -> Result<()> {
        let new_key = self.generate_key()?;
        let mut current = self
            .current_key
            .lock()
            .map_err(|_| SecurityError::KeyManagementError("key lock poisoned".to_string()))?;
        *current = new_key;
        self.generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    pub fn current_key(&self) -> Result<Vec<u8>> {
        let current = self
            .current_key
            .lock()
            .map_err(|_| SecurityError::KeyManagementError("key lock poisoned".to_string()))?;
        Ok(current.clone())
    }

    pub fn generation(&self) -> u32 {
        self.generation.load(Ordering::SeqCst)
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let manager = EncryptionManager::new();
        let key = vec![7u8; 32];
        let plaintext = b"sensitive data at rest";

        let ciphertext = manager.encrypt_at_rest(plaintext, &key).unwrap();
        assert_ne!(ciphertext, plaintext);

        let decrypted = manager.decrypt_at_rest(&ciphertext, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let manager = EncryptionManager::new();
        let key = vec![1u8; 32];
        let wrong_key = vec![2u8; 32];
        let ciphertext = manager.encrypt_at_rest(b"secret", &key).unwrap();

        assert!(manager.decrypt_at_rest(&ciphertext, &wrong_key).is_err());
    }

    #[test]
    fn test_rejects_wrong_key_length() {
        let manager = EncryptionManager::new();
        let short_key = vec![1u8; 8];
        assert!(manager.encrypt_at_rest(b"data", &short_key).is_err());
    }

    #[test]
    fn test_key_manager_rotate_changes_key() {
        let km = KeyManager::new();
        let before = km.current_key().unwrap();
        let generation_before = km.generation();

        km.rotate_key().unwrap();

        let after = km.current_key().unwrap();
        assert_ne!(before, after);
        assert_eq!(km.generation(), generation_before + 1);
    }

    #[test]
    fn test_generate_key_is_32_bytes() {
        let km = KeyManager::new();
        let key = km.generate_key().unwrap();
        assert_eq!(key.len(), 32);
    }
}
