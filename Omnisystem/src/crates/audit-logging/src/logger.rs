use crate::{AuditLog, AuditOutcome, LogIntegrity, RetentionPolicy, AuditQuery, AuditReport, AuditError, AuditResult};
use dashmap::DashMap;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;

pub struct AuditLogger {
    logs: Arc<DashMap<Uuid, AuditLog>>,
    integrity_chain: Arc<DashMap<Uuid, LogIntegrity>>,
    retention_policies: Arc<DashMap<String, RetentionPolicy>>,
    reports: Arc<DashMap<Uuid, AuditReport>>,
    /// The hash of the most recently appended log, so each new entry
    /// chains to the one before it (tamper-evident log).
    last_hash: Arc<Mutex<Option<String>>>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(DashMap::new()),
            integrity_chain: Arc::new(DashMap::new()),
            retention_policies: Arc::new(DashMap::new()),
            reports: Arc::new(DashMap::new()),
            last_hash: Arc::new(Mutex::new(None)),
        }
    }

    /// Compute a real SHA-256 digest over a log entry's content chained to
    /// the previous entry's hash, so any change to a log's content or to
    /// the chain ordering changes the resulting hash.
    fn compute_hash(log: &AuditLog, previous_hash: &Option<String>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(log.log_id.as_bytes());
        hasher.update(log.actor.as_bytes());
        hasher.update(log.action.as_bytes());
        hasher.update(log.resource.as_bytes());
        hasher.update(format!("{:?}", log.result).as_bytes());
        hasher.update(log.details.as_bytes());
        hasher.update(log.timestamp.timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        if let Some(prev) = previous_hash {
            hasher.update(prev.as_bytes());
        }
        format!("{:x}", hasher.finalize())
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

        let mut last_hash = self.last_hash.lock().unwrap();
        let hash = Self::compute_hash(&log, &last_hash);
        let integrity = LogIntegrity {
            integrity_id: Uuid::new_v4(),
            log_id: log.log_id,
            hash: hash.clone(),
            previous_hash: last_hash.clone(),
            timestamp: log.timestamp,
        };
        *last_hash = Some(hash);
        drop(last_hash);

        self.integrity_chain.insert(integrity.integrity_id, integrity);
        self.logs.insert(log.log_id, log.clone());
        Ok(log)
    }

    /// Recompute a log's hash from its current stored content and compare
    /// it against the hash recorded at append time. A mismatch means the
    /// log entry (or its position in the chain) was tampered with after
    /// the fact, rather than this always trivially returning true.
    pub async fn verify_integrity(&self, log_id: Uuid) -> AuditResult<bool> {
        let log = self.logs.get(&log_id).ok_or(AuditError::LogNotFound)?;
        let integrity = self
            .integrity_chain
            .iter()
            .find(|e| e.value().log_id == log_id)
            .map(|e| e.value().clone())
            .ok_or(AuditError::LogNotFound)?;

        let recomputed = Self::compute_hash(&log, &integrity.previous_hash);
        Ok(recomputed == integrity.hash)
    }

    /// Verify the entire hash chain end-to-end: every entry's
    /// `previous_hash` must match the hash actually produced by the entry
    /// immediately before it. Detects reordering or deletion of entries,
    /// not just per-entry content tampering.
    pub async fn verify_chain(&self) -> AuditResult<bool> {
        let mut entries: Vec<LogIntegrity> = self.integrity_chain.iter().map(|e| e.value().clone()).collect();
        entries.sort_by_key(|e| e.timestamp);

        let mut expected_previous: Option<String> = None;
        for entry in &entries {
            if entry.previous_hash != expected_previous {
                return Ok(false);
            }
            expected_previous = Some(entry.hash.clone());
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
    async fn test_verify_integrity_of_untampered_log() {
        let logger = AuditLogger::new();
        let log = logger.log_action("admin", "delete_user", "user_xyz", true).await.unwrap();

        let verified = logger.verify_integrity(log.log_id).await.unwrap();
        assert!(verified);
    }

    #[tokio::test]
    async fn test_verify_integrity_detects_tampering() {
        let logger = AuditLogger::new();
        let log = logger.log_action("admin", "delete_user", "user_xyz", true).await.unwrap();

        // Simulate an attacker mutating the stored log content after the
        // fact (e.g. changing a failed action to look like it succeeded).
        {
            let mut entry = logger.logs.get_mut(&log.log_id).unwrap();
            entry.action = "delete_user_covered_up".to_string();
        }

        let verified = logger.verify_integrity(log.log_id).await.unwrap();
        assert!(!verified, "tampered log content must fail integrity verification");
    }

    #[tokio::test]
    async fn test_verify_chain_links_entries_together() {
        let logger = AuditLogger::new();
        logger.log_action("user1", "read", "file1", true).await.unwrap();
        logger.log_action("user2", "write", "file2", true).await.unwrap();
        logger.log_action("user3", "delete", "file3", false).await.unwrap();

        assert!(logger.verify_chain().await.unwrap());
    }

    #[tokio::test]
    async fn test_verify_chain_detects_broken_link() {
        let logger = AuditLogger::new();
        logger.log_action("user1", "read", "file1", true).await.unwrap();
        let log2 = logger.log_action("user2", "write", "file2", true).await.unwrap();

        // Corrupt the recorded previous_hash for the second entry, as if
        // an entry were spliced out of the chain.
        {
            let integrity_id = logger
                .integrity_chain
                .iter()
                .find(|e| e.value().log_id == log2.log_id)
                .map(|e| *e.key())
                .unwrap();
            let mut entry = logger.integrity_chain.get_mut(&integrity_id).unwrap();
            entry.previous_hash = Some("forged_hash".to_string());
        }

        assert!(!logger.verify_chain().await.unwrap());
    }

    #[tokio::test]
    async fn test_verify_integrity_unknown_log_fails() {
        let logger = AuditLogger::new();
        let result = logger.verify_integrity(Uuid::new_v4()).await;
        assert!(matches!(result, Err(AuditError::LogNotFound)));
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

    #[tokio::test]
    async fn test_query_logs_filters_by_actor() {
        let logger = AuditLogger::new();
        logger.log_action("alice", "read", "file1", true).await.unwrap();
        logger.log_action("bob", "read", "file1", true).await.unwrap();

        let query = AuditQuery {
            query_id: Uuid::new_v4(),
            actor_filter: Some("alice".to_string()),
            action_filter: None,
            resource_filter: None,
            start_time: Utc::now() - chrono::Duration::hours(1),
            end_time: Utc::now() + chrono::Duration::hours(1),
        };

        let results = logger.query_logs(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actor, "alice");
    }
}
