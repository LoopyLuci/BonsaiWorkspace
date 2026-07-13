use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Secret {
    pub secret_id: Uuid,
    pub name: String,
    pub secret_type: SecretType,
    pub encrypted_value: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub last_rotated: DateTime<Utc>,
    pub expiry_date: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SecretType {
    ApiKey,
    DatabasePassword,
    PrivateKey,
    Token,
    Certificate,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RotationPolicy {
    pub policy_id: Uuid,
    pub secret_name: String,
    pub rotation_interval_days: u32,
    pub next_rotation: DateTime<Utc>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncryptionKey {
    pub key_id: Uuid,
    pub key_name: String,
    pub algorithm: String,
    pub key_size: u32,
    pub created_at: DateTime<Utc>,
    pub rotated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessLog {
    pub log_id: Uuid,
    pub secret_id: Uuid,
    pub accessor_id: String,
    pub access_type: AccessType,
    pub timestamp: DateTime<Utc>,
    pub granted: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum AccessType {
    Read,
    Write,
    Rotate,
    Delete,
}
