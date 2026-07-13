use crate::{ResourceQuota, ResourceUsage, QuotaError, QuotaResult, PriorityClass, EnforcementAction, QuotaEnforcement};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct QuotaManager {
    quotas: Arc<DashMap<String, ResourceQuota>>,
    usage: Arc<DashMap<String, ResourceUsage>>,
    enforcements: Arc<DashMap<Uuid, QuotaEnforcement>>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: Arc::new(DashMap::new()),
            usage: Arc::new(DashMap::new()),
            enforcements: Arc::new(DashMap::new()),
        }
    }

    pub async fn set_quota(&self, quota: &ResourceQuota) -> QuotaResult<()> {
        self.quotas.insert(quota.tenant_id.clone(), quota.clone());
        Ok(())
    }

    pub async fn get_quota(&self, tenant_id: &str) -> QuotaResult<ResourceQuota> {
        self.quotas
            .get(tenant_id)
            .map(|q| q.clone())
            .ok_or(QuotaError::QuotaNotFound)
    }

    pub async fn check_resource_availability(
        &self,
        tenant_id: &str,
        cpu: u32,
        memory_mb: u64,
    ) -> QuotaResult<bool> {
        if let Some(quota) = self.quotas.get(tenant_id) {
            Ok(quota.cpu_cores >= cpu && quota.memory_mb >= memory_mb)
        } else {
            Err(QuotaError::TenantNotFound)
        }
    }

    pub async fn record_usage(
        &self,
        tenant_id: &str,
        cpu: f32,
        memory_mb: u64,
        storage_gb: u64,
        network_mbps: u32,
    ) -> QuotaResult<()> {
        let usage = ResourceUsage {
            tenant_id: tenant_id.to_string(),
            cpu_cores_used: cpu,
            memory_mb_used: memory_mb,
            storage_gb_used: storage_gb,
            network_mbps_used: network_mbps,
        };

        self.usage.insert(tenant_id.to_string(), usage);
        Ok(())
    }

    pub async fn get_usage(&self, tenant_id: &str) -> QuotaResult<ResourceUsage> {
        self.usage
            .get(tenant_id)
            .map(|u| u.clone())
            .ok_or(QuotaError::TenantNotFound)
    }

    pub async fn enforce_quota(&self, tenant_id: &str) -> QuotaResult<EnforcementAction> {
        let quota = self.get_quota(tenant_id).await?;
        let usage = self.get_usage(tenant_id).await?;

        let cpu_ratio = usage.cpu_cores_used / quota.cpu_cores as f32;
        let memory_ratio = usage.memory_mb_used as f32 / quota.memory_mb as f32;

        if cpu_ratio > 1.0 || memory_ratio > 1.0 {
            let enforcement = QuotaEnforcement {
                enforcement_id: Uuid::new_v4(),
                tenant_id: tenant_id.to_string(),
                action: EnforcementAction::Deny,
                reason: "Quota exceeded".to_string(),
            };
            self.enforcements.insert(enforcement.enforcement_id, enforcement);
            Ok(EnforcementAction::Deny)
        } else if cpu_ratio > 0.9 || memory_ratio > 0.9 {
            Ok(EnforcementAction::Throttle)
        } else {
            Ok(EnforcementAction::Allow)
        }
    }

    pub fn quota_count(&self) -> usize {
        self.quotas.len()
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_quota() {
        let manager = QuotaManager::new();
        let quota = ResourceQuota {
            quota_id: Uuid::new_v4(),
            tenant_id: "tenant1".to_string(),
            cpu_cores: 4,
            memory_mb: 8192,
            storage_gb: 100,
            network_mbps: 1000,
            active: true,
        };

        manager.set_quota(&quota).await.unwrap();
        assert_eq!(manager.quota_count(), 1);
    }

    #[tokio::test]
    async fn test_get_quota() {
        let manager = QuotaManager::new();
        let quota = ResourceQuota {
            quota_id: Uuid::new_v4(),
            tenant_id: "tenant1".to_string(),
            cpu_cores: 8,
            memory_mb: 16384,
            storage_gb: 500,
            network_mbps: 5000,
            active: true,
        };

        manager.set_quota(&quota).await.unwrap();
        let retrieved = manager.get_quota("tenant1").await.unwrap();
        assert_eq!(retrieved.cpu_cores, 8);
    }

    #[tokio::test]
    async fn test_check_availability() {
        let manager = QuotaManager::new();
        let quota = ResourceQuota {
            quota_id: Uuid::new_v4(),
            tenant_id: "tenant1".to_string(),
            cpu_cores: 4,
            memory_mb: 8192,
            storage_gb: 100,
            network_mbps: 1000,
            active: true,
        };

        manager.set_quota(&quota).await.unwrap();
        let available = manager.check_resource_availability("tenant1", 2, 4096).await.unwrap();
        assert!(available);
    }

    #[tokio::test]
    async fn test_enforce_quota() {
        let manager = QuotaManager::new();
        let quota = ResourceQuota {
            quota_id: Uuid::new_v4(),
            tenant_id: "tenant1".to_string(),
            cpu_cores: 4,
            memory_mb: 8192,
            storage_gb: 100,
            network_mbps: 1000,
            active: true,
        };

        manager.set_quota(&quota).await.unwrap();
        manager.record_usage("tenant1", 2.0, 4096, 50, 500).await.unwrap();

        let action = manager.enforce_quota("tenant1").await.unwrap();
        assert_eq!(action, EnforcementAction::Allow);
    }
}
