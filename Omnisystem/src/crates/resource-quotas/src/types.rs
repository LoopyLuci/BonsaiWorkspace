use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub quota_id: Uuid,
    pub tenant_id: String,
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub storage_gb: u64,
    pub network_mbps: u32,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub tenant_id: String,
    pub cpu_cores_used: f32,
    pub memory_mb_used: u64,
    pub storage_gb_used: u64,
    pub network_mbps_used: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuotaLimit {
    pub limit_id: Uuid,
    pub resource_type: String,
    pub hard_limit: u64,
    pub soft_limit: u64,
    pub enforcement_level: EnforcementLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum EnforcementLevel {
    Warn = 0,
    Throttle = 1,
    Block = 2,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantAllocation {
    pub tenant_id: String,
    pub allocated_resources: Vec<(String, u64)>,
    pub priority: u32,
    pub priority_class: PriorityClass,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum PriorityClass {
    Free = 0,
    Standard = 1,
    Premium = 2,
    Enterprise = 3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuotaEnforcement {
    pub enforcement_id: Uuid,
    pub tenant_id: String,
    pub action: EnforcementAction,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum EnforcementAction {
    Allow,
    Throttle,
    Deny,
}
