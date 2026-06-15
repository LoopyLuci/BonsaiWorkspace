use crate::{EncryptionKey, EncryptionError, EncryptionResult, KeyRotationPolicy};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct KeyManager {
    keys: Arc<DashMap<String, EncryptionKey>>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_key(&self, key: &EncryptionKey) -> EncryptionResult<()> {
        self.keys.insert(key.key_id.clone(), key.clone());
        Ok(())
    }

    pub async fn get_key(&self, key_id: &str) -> EncryptionResult<EncryptionKey> {
        self.keys
            .get(key_id)
            .map(|entry| entry.clone())
            .ok_or(EncryptionError::KeyNotFound)
    }

    pub async fn rotate_key(&self, _policy: &KeyRotationPolicy) -> EncryptionResult<()> {
        Ok(())
    }

    pub async fn deactivate_key(&self, key_id: &str) -> EncryptionResult<()> {
        if let Some(mut key) = self.keys.get_mut(key_id) {
            key.is_active = false;
            Ok(())
        } else {
            Err(EncryptionError::KeyNotFound)
        }
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
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

    #[tokio::test]
    async fn test_create_key() {
        let manager = KeyManager::new();
        let key = EncryptionKey {
            key_id: "key-1".to_string(),
            algorithm: "AES-256".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            is_active: true,
        };

        manager.create_key(&key).await.unwrap();
        assert_eq!(manager.key_count(), 1);
    }

    #[tokio::test]
    async fn test_get_key() {
        let manager = KeyManager::new();
        let key = EncryptionKey {
            key_id: "key-1".to_string(),
            algorithm: "AES-256".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            is_active: true,
        };

        manager.create_key(&key).await.unwrap();
        let retrieved = manager.get_key("key-1").await.unwrap();
        assert_eq!(retrieved.algorithm, "AES-256");
    }

    #[tokio::test]
    async fn test_deactivate_key() {
        let manager = KeyManager::new();
        let key = EncryptionKey {
            key_id: "key-1".to_string(),
            algorithm: "AES-256".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            is_active: true,
        };

        manager.create_key(&key).await.unwrap();
        manager.deactivate_key("key-1").await.unwrap();
        let key = manager.get_key("key-1").await.unwrap();
        assert!(!key.is_active);
    }
}
