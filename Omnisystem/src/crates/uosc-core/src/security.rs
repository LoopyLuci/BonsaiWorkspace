/// Security Context and Access Control

use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use std::sync::Arc;

/// Security Context - Per-process security information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    pub principal_id: String,
    pub privilege_level: PrivilegeLevel,
    pub groups: Vec<String>,
    pub audit_enabled: bool,
}

/// Privilege Levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PrivilegeLevel {
    User = 0,
    System = 1,
    Supervisor = 2,
    Kernel = 3,
}

impl SecurityContext {
    pub fn new(principal_id: String) -> Self {
        SecurityContext {
            principal_id,
            privilege_level: PrivilegeLevel::User,
            groups: vec![],
            audit_enabled: false,
        }
    }

    pub fn with_privilege(mut self, level: PrivilegeLevel) -> Self {
        self.privilege_level = level;
        self
    }

    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.groups = groups;
        self
    }

    pub fn can_perform(&self, required_level: PrivilegeLevel) -> bool {
        self.privilege_level >= required_level
    }
}

/// Access Control Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlEntry {
    pub subject: String,
    pub resource: String,
    pub action: String,
    pub allowed: bool,
}

/// Access Control List
pub struct AccessControlList {
    entries: Arc<DashMap<String, AccessControlEntry>>,
}

impl AccessControlList {
    pub fn new() -> Self {
        AccessControlList {
            entries: Arc::new(DashMap::new()),
        }
    }

    pub fn add_rule(&self, subject: String, resource: String, action: String, allowed: bool) {
        let key = format!("{}:{}:{}", subject, resource, action);
        let entry = AccessControlEntry {
            subject,
            resource,
            action,
            allowed,
        };
        self.entries.insert(key, entry);
    }

    pub fn check_access(&self, subject: &str, resource: &str, action: &str) -> bool {
        let key = format!("{}:{}:{}", subject, resource, action);
        self.entries
            .get(&key)
            .map(|entry| entry.allowed)
            .unwrap_or(false)
    }

    pub fn list_entries(&self) -> Vec<AccessControlEntry> {
        self.entries.iter().map(|entry| entry.value().clone()).collect()
    }
}

impl Default for AccessControlList {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AccessControlList {
    fn clone(&self) -> Self {
        AccessControlList {
            entries: Arc::clone(&self.entries),
        }
    }
}

/// Audit Log Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: u64,
    pub principal: String,
    pub action: String,
    pub resource: String,
    pub allowed: bool,
    pub details: String,
}

/// Audit Logger
pub struct AuditLogger {
    entries: Arc<DashMap<u64, AuditLogEntry>>,
    counter: Arc<std::sync::atomic::AtomicU64>,
}

impl AuditLogger {
    pub fn new() -> Self {
        AuditLogger {
            entries: Arc::new(DashMap::new()),
            counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn log(&self, principal: String, action: String, resource: String, allowed: bool, details: String) {
        let id = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let entry = AuditLogEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            principal,
            action,
            resource,
            allowed,
            details,
        };
        self.entries.insert(id, entry);
    }

    pub fn get_logs(&self) -> Vec<AuditLogEntry> {
        self.entries.iter().map(|entry| entry.value().clone()).collect()
    }

    pub fn get_logs_for_principal(&self, principal: &str) -> Vec<AuditLogEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.value().principal == principal)
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AuditLogger {
    fn clone(&self) -> Self {
        AuditLogger {
            entries: Arc::clone(&self.entries),
            counter: Arc::new(std::sync::atomic::AtomicU64::new(
                self.counter.load(std::sync::atomic::Ordering::SeqCst),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_context() {
        let ctx = SecurityContext::new("user1".to_string())
            .with_privilege(PrivilegeLevel::User)
            .with_groups(vec!["admin".to_string()]);

        assert_eq!(ctx.principal_id, "user1");
        assert_eq!(ctx.privilege_level, PrivilegeLevel::User);
        assert_eq!(ctx.groups.len(), 1);
    }

    #[test]
    fn test_privilege_checking() {
        let ctx = SecurityContext::new("user1".to_string()).with_privilege(PrivilegeLevel::System);

        assert!(ctx.can_perform(PrivilegeLevel::User));
        assert!(ctx.can_perform(PrivilegeLevel::System));
        assert!(!ctx.can_perform(PrivilegeLevel::Kernel));
    }

    #[test]
    fn test_acl() {
        let acl = AccessControlList::new();
        acl.add_rule("user1".to_string(), "file.txt".to_string(), "read".to_string(), true);
        acl.add_rule("user1".to_string(), "file.txt".to_string(), "write".to_string(), false);

        assert!(acl.check_access("user1", "file.txt", "read"));
        assert!(!acl.check_access("user1", "file.txt", "write"));
        assert!(!acl.check_access("user2", "file.txt", "read"));
    }

    #[test]
    fn test_audit_logging() {
        let logger = AuditLogger::new();
        logger.log(
            "user1".to_string(),
            "read".to_string(),
            "file.txt".to_string(),
            true,
            "Read file successfully".to_string(),
        );

        let logs = logger.get_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].principal, "user1");
    }
}
