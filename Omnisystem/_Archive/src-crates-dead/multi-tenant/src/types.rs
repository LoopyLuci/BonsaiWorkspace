use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tenant {
    pub tenant_id: String,
    pub name: String,
    pub status: String,
    pub max_users: u32,
    pub max_storage_gb: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: String,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub policy_id: String,
    pub tenant_id: String,
    pub resource: String,
    pub action: String,
    pub effect: String,
}
