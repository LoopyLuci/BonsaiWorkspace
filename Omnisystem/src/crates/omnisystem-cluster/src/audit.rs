/// Audit Logging System
///
/// Immutable audit trail for compliance (SOC2, HIPAA, GDPR)

use crate::Result;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

/// Audit event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditEventType {
    // User management
    UserCreated,
    UserDeleted,
    RoleChanged,

    // Data access
    DataRead,
    DataWrite,
    DataDelete,

    // Cluster operations
    NodeJoined,
    NodeLeft,
    LeaderElected,
    BackupCreated,
    BackupRestored,

    // Security
    CertificateRotated,
    EncryptionKeyRotated,
    AccessDenied,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub event_id: String,
    pub timestamp: u64,
    pub event_type: AuditEventType,
    pub user_id: String,
    pub resource: String,
    pub action: String,
    pub result: AuditResult,
    pub details: String,
}

/// Audit result
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditResult {
    Success,
    Failure,
    Warning,
}

/// Audit logger
pub struct AuditLogger {
    log_entries: Vec<AuditLogEntry>,
    retention_days: u32,
}

impl AuditLogger {
    /// Create audit logger
    pub fn new(retention_days: u32) -> Result<Self> {
        info!(
            "Initializing Audit Logger with {} day retention",
            retention_days
        );
        Ok(Self {
            log_entries: Vec::new(),
            retention_days,
        })
    }

    /// Log event
    pub fn log_event(
        &mut self,
        event_type: AuditEventType,
        user_id: String,
        resource: String,
        action: String,
        result: AuditResult,
        details: String,
    ) -> Result<()> {
        let entry = AuditLogEntry {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            event_type,
            user_id,
            resource,
            action,
            result,
            details,
        };

        info!(
            "Audit event: {} - {} - {}",
            entry.event_id, entry.event_type as u32, entry.result as u32
        );
        self.log_entries.push(entry);
        Ok(())
    }

    /// Get audit entries
    pub fn get_entries(&self, limit: usize) -> Vec<AuditLogEntry> {
        self.log_entries
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Query audit logs by event type
    pub fn query_by_type(&self, event_type: AuditEventType) -> Vec<AuditLogEntry> {
        self.log_entries
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Query audit logs by user
    pub fn query_by_user(&self, user_id: &str) -> Vec<AuditLogEntry> {
        self.log_entries
            .iter()
            .filter(|e| e.user_id == user_id)
            .cloned()
            .collect()
    }

    /// Get failed attempts
    pub fn get_failed_attempts(&self) -> Vec<AuditLogEntry> {
        self.log_entries
            .iter()
            .filter(|e| e.result == AuditResult::Failure)
            .cloned()
            .collect()
    }

    /// Prune old entries (older than retention period)
    pub fn prune_old_entries(&mut self) -> Result<u32> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (self.retention_days as u64 * 86400);

        let initial_len = self.log_entries.len();
        self.log_entries.retain(|e| e.timestamp > cutoff);
        let pruned = (initial_len - self.log_entries.len()) as u32;

        info!("Pruned {} old audit entries", pruned);
        Ok(pruned)
    }

    /// Export audit log (for external auditors)
    pub fn export_audit_log(&self) -> Result<Vec<u8>> {
        info!("Exporting audit log ({} entries)", self.log_entries.len());
        let json = serde_json::to_vec(&self.log_entries)?;
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_creation() {
        let mut logger = AuditLogger::new(90).unwrap();
        logger
            .log_event(
                AuditEventType::UserCreated,
                "admin".to_string(),
                "user:user1".to_string(),
                "create".to_string(),
                AuditResult::Success,
                "Created new user".to_string(),
            )
            .unwrap();

        let entries = logger.get_entries(10);
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_query_by_type() {
        let mut logger = AuditLogger::new(90).unwrap();
        logger
            .log_event(
                AuditEventType::UserCreated,
                "admin".to_string(),
                "user:user1".to_string(),
                "create".to_string(),
                AuditResult::Success,
                "".to_string(),
            )
            .unwrap();

        logger
            .log_event(
                AuditEventType::DataWrite,
                "user1".to_string(),
                "data:123".to_string(),
                "write".to_string(),
                AuditResult::Success,
                "".to_string(),
            )
            .unwrap();

        let user_events = logger.query_by_type(AuditEventType::UserCreated);
        assert_eq!(user_events.len(), 1);
    }

    #[test]
    fn test_failed_attempts() {
        let mut logger = AuditLogger::new(90).unwrap();
        logger
            .log_event(
                AuditEventType::AccessDenied,
                "user1".to_string(),
                "admin_panel".to_string(),
                "access".to_string(),
                AuditResult::Failure,
                "Permission denied".to_string(),
            )
            .unwrap();

        let failed = logger.get_failed_attempts();
        assert_eq!(failed.len(), 1);
    }
}
