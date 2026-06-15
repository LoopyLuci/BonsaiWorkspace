use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub policy_id: Uuid,
    pub name: String,
    pub source: String,
    pub destination: String,
    pub port: u16,
    pub protocol: String,
    pub action: Action,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Action {
    Allow,
    Deny,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MtlsPolicy {
    pub policy_id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub min_tls_version: String,
    pub cipher_suites: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Certificate {
    pub cert_id: Uuid,
    pub common_name: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_to: String,
    pub is_valid: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkSegment {
    pub segment_id: Uuid,
    pub name: String,
    pub cidr: String,
    pub isolation_level: IsolationLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum IsolationLevel {
    Low,
    Medium,
    High,
    Strict,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessControl {
    pub control_id: Uuid,
    pub service_a: String,
    pub service_b: String,
    pub allowed: bool,
    pub reason: Option<String>,
}
