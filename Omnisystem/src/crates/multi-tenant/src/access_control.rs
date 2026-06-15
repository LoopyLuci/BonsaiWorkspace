use crate::{AccessPolicy, TenantContext, TenantError, TenantResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct AccessControlManager {
    policies: Arc<DashMap<String, Vec<AccessPolicy>>>,
}

impl AccessControlManager {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
        }
    }

    pub async fn check_access(
        &self,
        _context: &TenantContext,
        resource: &str,
        action: &str,
    ) -> TenantResult<bool> {
        if resource.is_empty() || action.is_empty() {
            return Err(TenantError::AccessDenied);
        }
        Ok(true)
    }

    pub async fn add_policy(&self, policy: &AccessPolicy) -> TenantResult<()> {
        self.policies
            .entry(policy.tenant_id.clone())
            .or_insert_with(Vec::new)
            .push(policy.clone());
        Ok(())
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for AccessControlManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_access() {
        let manager = AccessControlManager::new();
        let context = TenantContext {
            tenant_id: "t1".to_string(),
            user_id: "u1".to_string(),
            roles: vec!["admin".to_string()],
        };

        let result = manager.check_access(&context, "/api/users", "read").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_policy() {
        let manager = AccessControlManager::new();
        let policy = AccessPolicy {
            policy_id: "p1".to_string(),
            tenant_id: "t1".to_string(),
            resource: "/api/users".to_string(),
            action: "read".to_string(),
            effect: "allow".to_string(),
        };

        manager.add_policy(&policy).await.unwrap();
        assert_eq!(manager.policy_count(), 1);
    }
}
