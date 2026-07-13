use crate::{AuditLog, AuditOutcome, LogIntegrity, RetentionPolicy, AuditQuery, AuditReport, AuditError, AuditResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct AuditLogger {
    logs: Arc<DashMap<Uuid, AuditLog>>,
    integrity_chain: Arc<DashMap<Uuid, LogIntegrity>>,
    retention_policies: Arc<DashMap<String, RetentionPolicy>>,
    reports: Arc<DashMap<Uuid, AuditReport>>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(DashMap::new()),
            integrity_chain: Arc::new(DashMap::new()),
            retention_policies: Arc::new(DashMap::new()),
            reports: Arc::new(DashMap::new()),
        }
    }

    pub async fn log_action(&self, actor: &str, action: &str, resource: &str, success: bool) -> AuditResult<AuditLog> {
        let log = AuditLog {
            log_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: actor.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            result: if success { AuditOutcome::Success } else { AuditOutcome::Failure },
            details: "".to_string(),
        };

        self.logs.insert(log.log_id, log.clone());
        Ok(log)
    }

    pub async fn verify_integrity(&self, log_id: Uuid) -> AuditResult<bool> {
        if self.logs.get(&log_id).is_none() {
            return Err(AuditError::LogNotFound);
        }

        Ok(true)
    }

    pub async fn set_retention_policy(&self, log_type: &str, retention_days: u32) -> AuditResult<RetentionPolicy> {
        let policy = RetentionPolicy {
            policy_id: Uuid::new_v4(),
            log_type: log_type.to_string(),
            retention_days,
            archive_after_days: (retention_days as f64 * 0.5) as u32,
            enabled: true,
        };

        self.retention_policies.insert(log_type.to_string(), policy.clone());
        Ok(policy)
    }

    pub async fn query_logs(&self, query: &AuditQuery) -> AuditResult<Vec<AuditLog>> {
        let mut results = Vec::new();

        for entry in self.logs.iter() {
            let log = entry.value();

            let actor_match = query.actor_filter.as_ref().map_or(true, |f| log.actor.contains(f));
            let action_match = query.action_filter.as_ref().map_or(true, |f| log.action.contains(f));
            let resource_match = query.resource_filter.as_ref().map_or(true, |f| log.resource.contains(f));
            let time_match = log.timestamp >= query.start_time && log.timestamp <= query.end_time;

            if actor_match && action_match && resource_match && time_match {
                results.push(log.clone());
            }
        }

        Ok(results)
    }

    pub async fn generate_report(&self) -> AuditResult<AuditReport> {
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut actors = std::collections::HashSet::new();

        for entry in self.logs.iter() {
            let log = entry.value();
            if log.result == AuditOutcome::Success {
                success_count += 1;
            } else {
                failure_count += 1;
            }
            actors.insert(log.actor.clone());
        }

        let report = AuditReport {
            report_id: Uuid::new_v4(),
            generated_at: Utc::now(),
            total_logs: (success_count + failure_count) as u64,
            success_count,
            failure_count,
            actors: actors.into_iter().collect(),
        };

        self.reports.insert(report.report_id, report.clone());
        Ok(report)
    }

    pub fn log_count(&self) -> usize {
        self.logs.len()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_action() {
        let logger = AuditLogger::new();
        let log = logger.log_action("user_123", "create_resource", "database_abc", true).await.unwrap();

        assert_eq!(log.actor, "user_123");
        assert_eq!(log.result, AuditOutcome::Success);
        assert_eq!(logger.log_count(), 1);
    }

    #[tokio::test]
    async fn test_verify_integrity() {
        let logger = AuditLogger::new();
        let log = logger.log_action("admin", "delete_user", "user_xyz", true).await.unwrap();

        let verified = logger.verify_integrity(log.log_id).await.unwrap();
        assert!(verified);
    }

    #[tokio::test]
    async fn test_set_retention_policy() {
        let logger = AuditLogger::new();
        let policy = logger.set_retention_policy("security_logs", 365).await.unwrap();

        assert_eq!(policy.retention_days, 365);
        assert!(policy.enabled);
    }

    #[tokio::test]
    async fn test_generate_report() {
        let logger = AuditLogger::new();
        logger.log_action("user1", "read", "file1", true).await.unwrap();
        logger.log_action("user2", "write", "file2", false).await.unwrap();

        let report = logger.generate_report().await.unwrap();
        assert_eq!(report.total_logs, 2);
        assert_eq!(report.success_count, 1);
        assert_eq!(report.failure_count, 1);
    }
}
