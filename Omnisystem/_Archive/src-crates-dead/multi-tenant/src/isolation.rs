use crate::{Tenant, TenantError, TenantResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct TenantIsolationManager {
    tenants: Arc<DashMap<String, Tenant>>,
}

impl TenantIsolationManager {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_tenant(&self, tenant: &Tenant) -> TenantResult<()> {
        self.tenants.insert(tenant.tenant_id.clone(), tenant.clone());
        Ok(())
    }

    pub async fn get_tenant(&self, tenant_id: &str) -> TenantResult<Tenant> {
        self.tenants
            .get(tenant_id)
            .map(|entry| entry.clone())
            .ok_or(TenantError::TenantNotFound)
    }

    pub async fn delete_tenant(&self, tenant_id: &str) -> TenantResult<()> {
        if self.tenants.remove(tenant_id).is_some() {
            Ok(())
        } else {
            Err(TenantError::TenantNotFound)
        }
    }

    pub fn tenant_count(&self) -> usize {
        self.tenants.len()
    }
}

impl Default for TenantIsolationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_tenant() {
        let manager = TenantIsolationManager::new();
        let tenant = Tenant {
            tenant_id: "t1".to_string(),
            name: "Acme Corp".to_string(),
            status: "active".to_string(),
            max_users: 100,
            max_storage_gb: 1000,
        };

        manager.create_tenant(&tenant).await.unwrap();
        assert_eq!(manager.tenant_count(), 1);
    }

    #[tokio::test]
    async fn test_get_tenant() {
        let manager = TenantIsolationManager::new();
        let tenant = Tenant {
            tenant_id: "t1".to_string(),
            name: "Acme Corp".to_string(),
            status: "active".to_string(),
            max_users: 100,
            max_storage_gb: 1000,
        };

        manager.create_tenant(&tenant).await.unwrap();
        let retrieved = manager.get_tenant("t1").await.unwrap();
        assert_eq!(retrieved.name, "Acme Corp");
    }
}
