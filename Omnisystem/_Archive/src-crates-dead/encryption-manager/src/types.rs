use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptionKey {
    pub key_id: String,
    pub algorithm: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptedData {
    pub ciphertext: String,
    pub key_id: String,
    pub algorithm: String,
    pub iv: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyRotationPolicy {
    pub rotation_interval_days: u32,
    pub max_key_age_days: u32,
}
