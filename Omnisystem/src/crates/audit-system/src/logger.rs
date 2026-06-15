use crate::{AuditEvent, AuditError, AuditResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct AuditLogger {
    events: Arc<DashMap<String, Vec<AuditEvent>>>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
        }
    }

    pub async fn log_event(&self, event: &AuditEvent) -> AuditResult<()> {
        self.events
            .entry(event.user_id.clone())
            .or_insert_with(Vec::new)
            .push(event.clone());
        Ok(())
    }

    pub async fn get_events(&self, user_id: &str) -> AuditResult<Vec<AuditEvent>> {
        if let Some(events) = self.events.get(user_id) {
            Ok(events.clone())
        } else {
            Err(AuditError::QueryFailed)
        }
    }

    pub fn event_count(&self) -> usize {
        self.events.iter().map(|entry| entry.value().len()).sum()
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
    use chrono::Utc;

    #[tokio::test]
    async fn test_log_event() {
        let logger = AuditLogger::new();
        let event = AuditEvent {
            event_id: "e1".to_string(),
            user_id: "u1".to_string(),
            action: "login".to_string(),
            resource: "/api/auth".to_string(),
            timestamp: Utc::now(),
            status: "success".to_string(),
        };

        logger.log_event(&event).await.unwrap();
        assert_eq!(logger.event_count(), 1);
    }

    #[tokio::test]
    async fn test_get_events() {
        let logger = AuditLogger::new();
        let event = AuditEvent {
            event_id: "e1".to_string(),
            user_id: "u1".to_string(),
            action: "login".to_string(),
            resource: "/api/auth".to_string(),
            timestamp: Utc::now(),
            status: "success".to_string(),
        };

        logger.log_event(&event).await.unwrap();
        let events = logger.get_events("u1").await.unwrap();
        assert_eq!(events.len(), 1);
    }
}
