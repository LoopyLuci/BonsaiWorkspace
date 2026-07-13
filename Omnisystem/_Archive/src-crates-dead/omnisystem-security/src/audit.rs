use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub user: String,
    pub action: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct AuditLog {
    entries: Arc<Mutex<Vec<AuditEntry>>>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record(&self, user: String, action: String) {
        let entry = AuditEntry {
            user,
            action,
            timestamp: chrono::Utc::now(),
        };
        self.entries.lock().push(entry);
    }

    pub fn entry_count(&self) -> usize {
        self.entries.lock().len()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_audit_log() {
        let log = AuditLog::new();
        log.record("user1".to_string(), "login".to_string());
        assert_eq!(log.entry_count(), 1);
    }
}
