use app_manager_core::AppId;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub app_id: AppId,
    pub action: String,
    pub actor: String,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub details: Option<String>,
}

pub struct AuditLogger {
    events: Arc<DashMap<usize, AuditEvent>>,
    event_counter: Arc<std::sync::atomic::AtomicUsize>,
}

impl AuditLogger {
    pub fn new() -> Self {
        AuditLogger {
            events: Arc::new(DashMap::new()),
            event_counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn log_event(&self, event: AuditEvent) {
        let id = self.event_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.events.insert(id, event.clone());
        tracing::info!("Audit event: {} - {}", event.app_id, event.action);
    }

    pub fn log(&self, app_id: AppId, action: &str, actor: &str, success: bool) {
        self.log_event(AuditEvent {
            app_id,
            action: action.to_string(),
            actor: actor.to_string(),
            timestamp: Utc::now(),
            success,
            details: None,
        });
    }

    pub fn log_with_details(&self, app_id: AppId, action: &str, actor: &str, success: bool, details: &str) {
        self.log_event(AuditEvent {
            app_id,
            action: action.to_string(),
            actor: actor.to_string(),
            timestamp: Utc::now(),
            success,
            details: Some(details.to_string()),
        });
    }

    pub fn get_events(&self) -> Vec<AuditEvent> {
        self.events.iter().map(|r| r.value().clone()).collect()
    }

    pub fn get_events_for_app(&self, app_id: &AppId) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|r| r.value().app_id == *app_id)
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn get_events_by_action(&self, action: &str) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|r| r.value().action == action)
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn get_failed_events(&self) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|r| !r.value().success)
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn clear_events(&self) {
        self.events.clear();
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn export_to_json(&self) -> Result<String, serde_json::Error> {
        let events: Vec<_> = self.get_events();
        serde_json::to_string(&events)
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

    #[test]
    fn test_audit_logger_creation() {
        let logger = AuditLogger::new();
        assert_eq!(logger.event_count(), 0);
    }

    #[test]
    fn test_log_event() {
        let logger = AuditLogger::new();
        logger.log(AppId::new("test").unwrap(), "install", "system", true);

        assert_eq!(logger.event_count(), 1);
    }

    #[test]
    fn test_get_events_for_app() {
        let logger = AuditLogger::new();
        let app_id = AppId::new("test").unwrap();

        logger.log(app_id.clone(), "install", "system", true);
        logger.log(app_id.clone(), "start", "system", true);
        logger.log(AppId::new("other").unwrap(), "install", "system", true);

        let events = logger.get_events_for_app(&app_id);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_get_failed_events() {
        let logger = AuditLogger::new();
        logger.log(AppId::new("test").unwrap(), "install", "system", true);
        logger.log(AppId::new("test").unwrap(), "start", "system", false);
        logger.log(AppId::new("test").unwrap(), "load", "system", false);

        let failed = logger.get_failed_events();
        assert_eq!(failed.len(), 2);
    }

    #[test]
    fn test_export_to_json() {
        let logger = AuditLogger::new();
        logger.log(AppId::new("test").unwrap(), "install", "system", true);

        let json = logger.export_to_json().unwrap();
        assert!(!json.is_empty());
    }
}
