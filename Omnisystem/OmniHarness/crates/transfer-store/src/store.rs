//! EncryptedStore — AES-256-GCM at-rest encryption with Argon2id key derivation.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"BONSAI01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const BLAKE3_TAG_LEN: usize = 32;
/// Argon2id params: 64 MiB, 3 iterations, 1 thread.
const ARGON2_M: u32 = 65536;
const ARGON2_T: u32 = 3;
const ARGON2_P: u32 = 1;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid magic bytes — not a Bonsai store file")]
    BadMagic,
    #[error("integrity check failed — file may be corrupted or tampered")]
    IntegrityFailed,
    #[error("decryption failed")]
    DecryptFailed,
    #[error("key derivation failed: {0}")]
    Kdf(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type StoreResult<T> = Result<T, StoreError>;

/// An encrypted persistent store backed by a single file.
pub struct EncryptedStore {
    path: PathBuf,
    passphrase: Zeroizing<Vec<u8>>,
}

impl EncryptedStore {
    /// Open (or create) a store at `path`, unlocked by `passphrase`.
    pub fn open(path: impl AsRef<Path>, passphrase: &[u8]) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            passphrase: Zeroizing::new(passphrase.to_vec()),
        }
    }

    /// Default platform path: `{data_dir}/bonsai/store.bin`
    pub fn default_path() -> PathBuf {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("bonsai").join("store.bin")
    }

    /// Serialize `value` to JSON, encrypt, and write to disk.
    pub fn save<T: Serialize>(&self, value: &T) -> StoreResult<()> {
        let plaintext = serde_json::to_vec(value)?;
        let ciphertext = self.encrypt(&plaintext)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, ciphertext)?;
        Ok(())
    }

    /// Load and decrypt, then deserialize to `T`.
    pub fn load<T: for<'de> Deserialize<'de>>(&self) -> StoreResult<T> {
        let raw = std::fs::read(&self.path)?;
        let plaintext = self.decrypt(&raw)?;
        let value = serde_json::from_slice(&plaintext)?;
        Ok(value)
    }

    /// Returns true if the backing file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Permanently delete the backing file.
    pub fn delete(&self) -> StoreResult<()> {
        std::fs::remove_file(&self.path)?;
        Ok(())
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn derive_key(&self, salt: &[u8; SALT_LEN]) -> StoreResult<Zeroizing<[u8; 32]>> {
        let params = Params::new(ARGON2_M, ARGON2_T, ARGON2_P, Some(32))
            .map_err(|e| StoreError::Kdf(e.to_string()))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut key = Zeroizing::new([0u8; 32]);
        argon2
            .hash_password_into(&self.passphrase, salt, key.as_mut())
            .map_err(|e| StoreError::Kdf(e.to_string()))?;
        Ok(key)
    }

    fn encrypt(&self, plaintext: &[u8]) -> StoreResult<Vec<u8>> {
        let mut salt = [0u8; SALT_LEN];
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut salt);
        rand::thread_rng().fill_bytes(&mut nonce_bytes);

        let key_raw = self.derive_key(&salt)?;
        let key = Key::<Aes256Gcm>::from_slice(key_raw.as_ref());
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ct = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| StoreError::DecryptFailed)?;

        // Build file: MAGIC | SALT | NONCE | CIPHERTEXT | BLAKE3_TAG
        let mut file = Vec::with_capacity(8 + SALT_LEN + NONCE_LEN + ct.len() + BLAKE3_TAG_LEN);
        file.extend_from_slice(MAGIC);
        file.extend_from_slice(&salt);
        file.extend_from_slice(&nonce_bytes);
        file.extend_from_slice(&ct);

        // Integrity tag over everything so far
        let tag: [u8; 32] = *blake3::hash(&file).as_bytes();
        file.extend_from_slice(&tag);

        Ok(file)
    }

    fn decrypt(&self, raw: &[u8]) -> StoreResult<Vec<u8>> {
        let min_len = 8 + SALT_LEN + NONCE_LEN + BLAKE3_TAG_LEN;
        if raw.len() < min_len {
            return Err(StoreError::BadMagic);
        }
        if &raw[..8] != MAGIC {
            return Err(StoreError::BadMagic);
        }

        let tag_offset = raw.len() - BLAKE3_TAG_LEN;
        let body = &raw[..tag_offset];
        let stored_tag = &raw[tag_offset..];
        let computed_tag: [u8; 32] = *blake3::hash(body).as_bytes();
        if computed_tag.as_ref() != stored_tag {
            return Err(StoreError::IntegrityFailed);
        }

        let mut offset = 8;
        let salt: [u8; SALT_LEN] = raw[offset..offset + SALT_LEN].try_into().unwrap();
        offset += SALT_LEN;
        let nonce_bytes: [u8; NONCE_LEN] = raw[offset..offset + NONCE_LEN].try_into().unwrap();
        offset += NONCE_LEN;
        let ct = &raw[offset..tag_offset];

        let key_raw = self.derive_key(&salt)?;
        let key = Key::<Aes256Gcm>::from_slice(key_raw.as_ref());
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        cipher
            .decrypt(nonce, ct)
            .map_err(|_| StoreError::DecryptFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trip() {
        let dir = std::env::temp_dir().join(format!("bonsai_store_test_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bin");

        let store = EncryptedStore::open(&path, b"test-passphrase");
        let data = json!({"hello": "world", "count": 42});
        store.save(&data).unwrap();

        let loaded: serde_json::Value = store.load().unwrap();
        assert_eq!(loaded["hello"], "world");
        assert_eq!(loaded["count"], 42);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn wrong_passphrase_fails() {
        let dir = std::env::temp_dir().join(format!("bonsai_store_test_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bin");

        let store = EncryptedStore::open(&path, b"correct-passphrase");
        store.save(&json!({"secret": true})).unwrap();

        let bad = EncryptedStore::open(&path, b"wrong-passphrase");
        assert!(bad.load::<serde_json::Value>().is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn tamper_detected() {
        let dir = std::env::temp_dir().join(format!("bonsai_store_test_{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bin");

        let store = EncryptedStore::open(&path, b"passphrase");
        store.save(&json!({"x": 1})).unwrap();

        // Flip a byte in the ciphertext region
        let mut raw = std::fs::read(&path).unwrap();
        raw[40] ^= 0xFF;
        std::fs::write(&path, &raw).unwrap();

        assert!(matches!(
            store.load::<serde_json::Value>(),
            Err(StoreError::IntegrityFailed)
        ));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
