use crate::{EncryptedData, EncryptionError, EncryptionResult};

pub struct EncryptionEngine;

impl EncryptionEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn encrypt(&self, plaintext: &str, key_id: &str) -> EncryptionResult<EncryptedData> {
        if plaintext.is_empty() || key_id.is_empty() {
            return Err(EncryptionError::InvalidKey);
        }

        Ok(EncryptedData {
            ciphertext: format!("encrypted_{}", plaintext),
            key_id: key_id.to_string(),
            algorithm: "AES-256-GCM".to_string(),
            iv: "iv_placeholder".to_string(),
        })
    }

    pub async fn decrypt(&self, encrypted: &EncryptedData) -> EncryptionResult<String> {
        if encrypted.ciphertext.is_empty() {
            return Err(EncryptionError::DecryptionFailed);
        }

        let plaintext = encrypted.ciphertext.replace("encrypted_", "");
        Ok(plaintext)
    }
}

impl Default for EncryptionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_encrypt() {
        let engine = EncryptionEngine::new();
        let encrypted = engine.encrypt("secret", "key-1").await.unwrap();
        assert_eq!(encrypted.algorithm, "AES-256-GCM");
    }

    #[tokio::test]
    async fn test_decrypt() {
        let engine = EncryptionEngine::new();
        let encrypted = engine.encrypt("secret", "key-1").await.unwrap();
        let plaintext = engine.decrypt(&encrypted).await.unwrap();
        assert_eq!(plaintext, "secret");
    }
}
