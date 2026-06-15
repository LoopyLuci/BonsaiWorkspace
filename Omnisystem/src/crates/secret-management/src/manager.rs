use crate::{Secret, SecretType, RotationPolicy, EncryptionKey, AccessLog, AccessType, SecretError, SecretResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct SecretManager {
    secrets: Arc<DashMap<Uuid, Secret>>,
    rotation_policies: Arc<DashMap<String, RotationPolicy>>,
    encryption_keys: Arc<DashMap<Uuid, EncryptionKey>>,
    access_logs: Arc<DashMap<Uuid, AccessLog>>,
}

impl SecretManager {
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(DashMap::new()),
            rotation_policies: Arc::new(DashMap::new()),
            encryption_keys: Arc::new(DashMap::new()),
            access_logs: Arc::new(DashMap::new()),
        }
    }

    pub async fn store_secret(&self, name: &str, secret_type: SecretType, value: &[u8]) -> SecretResult<Secret> {
        let secret = Secret {
            secret_id: Uuid::new_v4(),
            name: name.to_string(),
            secret_type,
            encrypted_value: value.to_vec(),
            created_at: Utc::now(),
            last_rotated: Utc::now(),
            expiry_date: None,
        };

        self.secrets.insert(secret.secret_id, secret.clone());
        Ok(secret)
    }

    pub async fn retrieve_secret(&self, secret_id: Uuid) -> SecretResult<Secret> {
        self.secrets
            .get(&secret_id)
            .map(|s| s.clone())
            .ok_or(SecretError::SecretNotFound)
    }

    pub async fn rotate_secret(&self, secret_id: Uuid, new_value: &[u8]) -> SecretResult<()> {
        if let Some(mut entry) = self.secrets.get_mut(&secret_id) {
            entry.encrypted_value = new_value.to_vec();
            entry.last_rotated = Utc::now();
        } else {
            return Err(SecretError::SecretNotFound);
        }

        Ok(())
    }

    pub async fn set_rotation_policy(&self, secret_name: &str, interval_days: u32) -> SecretResult<RotationPolicy> {
        let policy = RotationPolicy {
            policy_id: Uuid::new_v4(),
            secret_name: secret_name.to_string(),
            rotation_interval_days: interval_days,
            next_rotation: Utc::now() + chrono::Duration::days(interval_days as i64),
            enabled: true,
        };

        self.rotation_policies.insert(secret_name.to_string(), policy.clone());
        Ok(policy)
    }

    pub async fn create_encryption_key(&self, name: &str, algorithm: &str, key_size: u32) -> SecretResult<EncryptionKey> {
        let key = EncryptionKey {
            key_id: Uuid::new_v4(),
            key_name: name.to_string(),
            algorithm: algorithm.to_string(),
            key_size,
            created_at: Utc::now(),
            rotated_at: None,
        };

        self.encryption_keys.insert(key.key_id, key.clone());
        Ok(key)
    }

    pub async fn log_access(&self, secret_id: Uuid, accessor_id: &str, access_type: AccessType, granted: bool) -> SecretResult<()> {
        let log = AccessLog {
            log_id: Uuid::new_v4(),
            secret_id,
            accessor_id: accessor_id.to_string(),
            access_type,
            timestamp: Utc::now(),
            granted,
        };

        self.access_logs.insert(log.log_id, log);
        Ok(())
    }

    pub fn secret_count(&self) -> usize {
        self.secrets.len()
    }
}

impl Default for SecretManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_secret() {
        let manager = SecretManager::new();
        let secret = manager.store_secret("db_password", SecretType::DatabasePassword, b"secret123").await.unwrap();

        assert_eq!(secret.name, "db_password");
        assert_eq!(manager.secret_count(), 1);
    }

    #[tokio::test]
    async fn test_retrieve_secret() {
        let manager = SecretManager::new();
        let secret = manager.store_secret("api_key", SecretType::ApiKey, b"key_xyz").await.unwrap();

        let retrieved = manager.retrieve_secret(secret.secret_id).await.unwrap();
        assert_eq!(retrieved.name, "api_key");
    }

    #[tokio::test]
    async fn test_rotate_secret() {
        let manager = SecretManager::new();
        let secret = manager.store_secret("token", SecretType::Token, b"old_token").await.unwrap();

        manager.rotate_secret(secret.secret_id, b"new_token").await.unwrap();
    }

    #[tokio::test]
    async fn test_set_rotation_policy() {
        let manager = SecretManager::new();
        let policy = manager.set_rotation_policy("critical_secret", 30).await.unwrap();

        assert_eq!(policy.rotation_interval_days, 30);
        assert!(policy.enabled);
    }
}
