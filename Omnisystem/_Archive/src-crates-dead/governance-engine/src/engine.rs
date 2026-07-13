use crate::{GovernancePolicy, RetentionPolicy, ComplianceCheck, DataAccessLog, GovernanceError, GovernanceResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct GovernanceEngine {
    policies: Arc<DashMap<Uuid, GovernancePolicy>>,
    retention_policies: Arc<DashMap<Uuid, RetentionPolicy>>,
    compliance_checks: Arc<DashMap<Uuid, ComplianceCheck>>,
    access_logs: Arc<DashMap<Uuid, DataAccessLog>>,
}

impl GovernanceEngine {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
            retention_policies: Arc::new(DashMap::new()),
            compliance_checks: Arc::new(DashMap::new()),
            access_logs: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_policy(&self, policy: &GovernancePolicy) -> GovernanceResult<()> {
        self.policies.insert(policy.policy_id, policy.clone());
        Ok(())
    }

    pub async fn set_retention(&self, retention: &RetentionPolicy) -> GovernanceResult<()> {
        self.retention_policies.insert(retention.retention_id, retention.clone());
        Ok(())
    }

    pub async fn check_compliance(&self, check: &ComplianceCheck) -> GovernanceResult<()> {
        self.compliance_checks.insert(check.check_id, check.clone());
        Ok(())
    }

    pub async fn log_access(&self, log: &DataAccessLog) -> GovernanceResult<()> {
        self.access_logs.insert(log.log_id, log.clone());
        Ok(())
    }

    pub async fn enforce_policy(&self, dataset_id: Uuid, user_id: &str) -> GovernanceResult<bool> {
        for policy in self.policies.iter() {
            if policy.value().dataset_id == dataset_id {
                return Ok(true);
            }
        }

        Err(GovernanceError::PolicyNotFound)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }
}

impl Default for GovernanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_policy() {
        let engine = GovernanceEngine::new();
        let policy = GovernancePolicy {
            policy_id: Uuid::new_v4(),
            name: "pii_access".to_string(),
            dataset_id: Uuid::new_v4(),
            access_level: "restricted".to_string(),
        };

        engine.create_policy(&policy).await.unwrap();
        assert_eq!(engine.policy_count(), 1);
    }

    #[tokio::test]
    async fn test_set_retention() {
        let engine = GovernanceEngine::new();
        let retention = RetentionPolicy {
            retention_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            retention_days: 365,
            deletion_date: Utc::now() + chrono::Duration::days(365),
        };

        engine.set_retention(&retention).await.unwrap();
    }

    #[tokio::test]
    async fn test_check_compliance() {
        let engine = GovernanceEngine::new();
        let check = ComplianceCheck {
            check_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            compliance_status: "compliant".to_string(),
            checked_at: Utc::now(),
        };

        engine.check_compliance(&check).await.unwrap();
    }

    #[tokio::test]
    async fn test_log_access() {
        let engine = GovernanceEngine::new();
        let log = DataAccessLog {
            log_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            user_id: "user123".to_string(),
            access_time: Utc::now(),
        };

        engine.log_access(&log).await.unwrap();
    }
}
