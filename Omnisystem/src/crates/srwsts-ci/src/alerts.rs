//! Alert management with email/Slack notifications and escalation

use crate::errors::{CIError, CIResult};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Alert level
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
    Emergency,
}

impl AlertLevel {
    pub fn to_priority(&self) -> u8 {
        match self {
            AlertLevel::Info => 1,
            AlertLevel::Warning => 2,
            AlertLevel::Critical => 3,
            AlertLevel::Emergency => 4,
        }
    }
}

/// Alert channel
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum AlertChannel {
    Email,
    Slack,
    PagerDuty,
    Webhook(String),
}

/// Single alert event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub alert_id: String,
    pub level: AlertLevel,
    pub channel: AlertChannel,
    pub subject: String,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub delivery_status: AlertStatus,
    pub recipients: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Alert delivery status
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum AlertStatus {
    Pending,
    Delivered,
    Failed,
    Acknowledged,
    Escalated,
}

/// Alert manager
pub struct AlertManager {
    alerts: dashmap::DashMap<String, AlertEvent>,
    subscribers: dashmap::DashMap<AlertLevel, Vec<String>>,
    escalation_threshold: Duration,
}

impl AlertManager {
    /// Create new alert manager
    pub fn new() -> Self {
        Self {
            alerts: dashmap::DashMap::new(),
            subscribers: dashmap::DashMap::new(),
            escalation_threshold: Duration::hours(24),
        }
    }

    /// Create alert with custom escalation threshold
    pub fn with_escalation_threshold(threshold: Duration) -> Self {
        Self {
            alerts: dashmap::DashMap::new(),
            subscribers: dashmap::DashMap::new(),
            escalation_threshold: threshold,
        }
    }

    /// Subscribe to alerts at specific level
    pub fn subscribe(&self, level: AlertLevel, email: &str) -> CIResult<()> {
        let mut subscribers = self.subscribers
            .entry(level)
            .or_insert_with(Vec::new);

        if !subscribers.contains(&email.to_string()) {
            subscribers.push(email.to_string());
            info!("Subscribed {} to {:?} alerts", email, level);
        }
        Ok(())
    }

    /// Unsubscribe from alerts
    pub fn unsubscribe(&self, level: AlertLevel, email: &str) -> CIResult<()> {
        if let Some(mut subscribers) = self.subscribers.get_mut(&level) {
            subscribers.retain(|e| e != email);
            info!("Unsubscribed {} from {:?} alerts", email, level);
        }
        Ok(())
    }

    /// Send alert
    pub async fn send_alert(
        &self,
        level: AlertLevel,
        channel: AlertChannel,
        subject: &str,
        message: &str,
        recipients: Vec<String>,
    ) -> CIResult<String> {
        let alert_id = uuid::Uuid::new_v4().to_string();

        let alert = AlertEvent {
            alert_id: alert_id.clone(),
            level,
            channel: channel.clone(),
            subject: subject.to_string(),
            message: message.to_string(),
            triggered_at: Utc::now(),
            delivered_at: None,
            delivery_status: AlertStatus::Pending,
            recipients: recipients.clone(),
            metadata: HashMap::new(),
        };

        self.alerts.insert(alert_id.clone(), alert.clone());

        // Simulate delivery
        match self.deliver_alert(&alert).await {
            Ok(_) => {
                if let Some(mut alert) = self.alerts.get_mut(&alert_id) {
                    alert.delivered_at = Some(Utc::now());
                    alert.delivery_status = AlertStatus::Delivered;
                }
                info!(
                    "Alert {} sent to {:?} via {:?}",
                    alert_id, recipients, channel
                );
                Ok(alert_id)
            }
            Err(e) => {
                warn!("Failed to deliver alert {}: {}", alert_id, e);
                if let Some(mut alert) = self.alerts.get_mut(&alert_id) {
                    alert.delivery_status = AlertStatus::Failed;
                }
                Err(CIError::AlertDeliveryFailed {
                    target: format!("{:?}", channel),
                    reason: e.to_string(),
                })
            }
        }
    }

    /// Acknowledge alert
    pub fn acknowledge_alert(&self, alert_id: &str, _acknowledger: &str) -> CIResult<()> {
        if let Some(mut alert) = self.alerts.get_mut(alert_id) {
            alert.delivery_status = AlertStatus::Acknowledged;
            info!("Alert {} acknowledged", alert_id);
            Ok(())
        } else {
            Err(CIError::Internal(format!("Alert {} not found", alert_id)))
        }
    }

    /// Escalate alert
    pub fn escalate_alert(&self, alert_id: &str, reason: &str) -> CIResult<()> {
        if let Some(mut alert) = self.alerts.get_mut(alert_id) {
            alert.delivery_status = AlertStatus::Escalated;
            alert.metadata.insert("escalation_reason".to_string(), reason.to_string());
            alert.metadata.insert("escalated_at".to_string(), Utc::now().to_rfc3339());

            warn!("Alert {} escalated: {}", alert_id, reason);
            Ok(())
        } else {
            Err(CIError::Internal(format!("Alert {} not found", alert_id)))
        }
    }

    /// Check for alerts needing escalation
    pub fn get_unacknowledged_alerts(&self) -> Vec<AlertEvent> {
        self.alerts
            .iter()
            .filter(|entry| {
                entry.value().delivery_status != AlertStatus::Acknowledged
                    && entry.value().delivery_status != AlertStatus::Escalated
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get alerts for escalation (older than threshold)
    pub fn get_escalation_candidates(&self) -> Vec<AlertEvent> {
        let now = Utc::now();
        self.alerts
            .iter()
            .filter(|entry| {
                let alert = entry.value();
                let age = now - alert.triggered_at;
                age > self.escalation_threshold
                    && alert.delivery_status != AlertStatus::Acknowledged
                    && alert.delivery_status != AlertStatus::Escalated
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get alert by ID
    pub fn get_alert(&self, alert_id: &str) -> Option<AlertEvent> {
        self.alerts.get(alert_id).map(|a| a.clone())
    }

    /// Get all alerts
    pub fn get_all_alerts(&self) -> Vec<AlertEvent> {
        self.alerts
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get alerts by level
    pub fn get_alerts_by_level(&self, level: AlertLevel) -> Vec<AlertEvent> {
        self.alerts
            .iter()
            .filter(|entry| entry.value().level == level)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Send regression alert
    pub async fn send_regression_alert(
        &self,
        regression_details: &str,
        affected_metric: &str,
    ) -> CIResult<String> {
        let subject = format!("Regression Detected: {}", affected_metric);
        let message = format!(
            "A performance regression has been detected:\n{}\n\nAffected metric: {}",
            regression_details, affected_metric
        );

        // Get critical alert subscribers
        let recipients = self.subscribers
            .get(&AlertLevel::Critical)
            .map(|subs| subs.clone())
            .unwrap_or_default();

        if recipients.is_empty() {
            return Err(CIError::AlertDeliveryFailed {
                target: "email".to_string(),
                reason: "No subscribers configured".to_string(),
            });
        }

        self.send_alert(
            AlertLevel::Critical,
            AlertChannel::Email,
            &subject,
            &message,
            recipients,
        )
        .await
    }

    async fn deliver_alert(&self, alert: &AlertEvent) -> CIResult<()> {
        // Mock implementation - in real system would actually deliver
        match alert.channel {
            AlertChannel::Email => {
                info!("Mock: Sending email to {:?}", alert.recipients);
                Ok(())
            }
            AlertChannel::Slack => {
                info!("Mock: Posting to Slack: {}", alert.subject);
                Ok(())
            }
            AlertChannel::PagerDuty => {
                info!("Mock: Creating PagerDuty incident");
                Ok(())
            }
            AlertChannel::Webhook(ref url) => {
                info!("Mock: Posting to webhook: {}", url);
                Ok(())
            }
        }
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_alert() {
        let manager = AlertManager::new();

        let alert_id = manager
            .send_alert(
                AlertLevel::Critical,
                AlertChannel::Email,
                "Test Alert",
                "This is a test",
                vec!["admin@example.com".to_string()],
            )
            .await
            .unwrap();

        assert!(!alert_id.is_empty());
        let alert = manager.get_alert(&alert_id).unwrap();
        assert_eq!(alert.level, AlertLevel::Critical);
    }

    #[test]
    fn test_subscribe_to_alerts() {
        let manager = AlertManager::new();

        assert!(manager
            .subscribe(AlertLevel::Critical, "admin@example.com")
            .is_ok());

        let all_alerts = manager.get_all_alerts();
        assert_eq!(all_alerts.len(), 0); // No alerts sent yet
    }

    #[test]
    fn test_unsubscribe_from_alerts() {
        let manager = AlertManager::new();

        manager
            .subscribe(AlertLevel::Critical, "admin@example.com")
            .unwrap();
        manager
            .unsubscribe(AlertLevel::Critical, "admin@example.com")
            .unwrap();

        // Should not error on unsubscribe
    }

    #[tokio::test]
    async fn test_acknowledge_alert() {
        let manager = AlertManager::new();

        let alert_id = manager
            .send_alert(
                AlertLevel::Warning,
                AlertChannel::Email,
                "Warning",
                "Test",
                vec!["admin@example.com".to_string()],
            )
            .await
            .unwrap();

        manager.acknowledge_alert(&alert_id, "admin").unwrap();

        let alert = manager.get_alert(&alert_id).unwrap();
        assert_eq!(alert.delivery_status, AlertStatus::Acknowledged);
    }

    #[tokio::test]
    async fn test_escalate_alert() {
        let manager = AlertManager::new();

        let alert_id = manager
            .send_alert(
                AlertLevel::Critical,
                AlertChannel::Email,
                "Critical",
                "Test",
                vec!["admin@example.com".to_string()],
            )
            .await
            .unwrap();

        manager
            .escalate_alert(&alert_id, "Not acknowledged for 2 hours")
            .unwrap();

        let alert = manager.get_alert(&alert_id).unwrap();
        assert_eq!(alert.delivery_status, AlertStatus::Escalated);
    }

    #[tokio::test]
    async fn test_get_unacknowledged_alerts() {
        let manager = AlertManager::new();

        let alert_id1 = manager
            .send_alert(
                AlertLevel::Warning,
                AlertChannel::Email,
                "W1",
                "Test",
                vec!["admin@example.com".to_string()],
            )
            .await
            .unwrap();

        let alert_id2 = manager
            .send_alert(
                AlertLevel::Critical,
                AlertChannel::Email,
                "C1",
                "Test",
                vec!["admin@example.com".to_string()],
            )
            .await
            .unwrap();

        manager.acknowledge_alert(&alert_id1, "admin").unwrap();

        let unack = manager.get_unacknowledged_alerts();
        assert_eq!(unack.len(), 1);
        assert_eq!(unack[0].alert_id, alert_id2);
    }

    #[tokio::test]
    async fn test_get_alerts_by_level() {
        let manager = AlertManager::new();

        let _ = manager
            .send_alert(
                AlertLevel::Info,
                AlertChannel::Email,
                "Info",
                "Test",
                vec!["admin@example.com".to_string()],
            )
            .await;

        let _ = manager
            .send_alert(
                AlertLevel::Critical,
                AlertChannel::Email,
                "Critical",
                "Test",
                vec!["admin@example.com".to_string()],
            )
            .await;

        let critical = manager.get_alerts_by_level(AlertLevel::Critical);
        assert_eq!(critical.len(), 1);
        assert_eq!(critical[0].level, AlertLevel::Critical);
    }

    #[tokio::test]
    async fn test_send_regression_alert() {
        let manager = AlertManager::new();
        manager
            .subscribe(AlertLevel::Critical, "admin@example.com")
            .unwrap();

        let alert_id = manager
            .send_regression_alert("Latency increased by 50%", "latency_p99")
            .await
            .unwrap();

        assert!(!alert_id.is_empty());
    }

    #[test]
    fn test_alert_level_priority() {
        assert!(AlertLevel::Emergency.to_priority() > AlertLevel::Critical.to_priority());
        assert!(AlertLevel::Critical.to_priority() > AlertLevel::Warning.to_priority());
    }

    #[test]
    fn test_alert_status_equality() {
        assert_eq!(AlertStatus::Pending, AlertStatus::Pending);
        assert_ne!(AlertStatus::Pending, AlertStatus::Delivered);
    }

    #[tokio::test]
    async fn test_get_escalation_candidates() {
        let manager = AlertManager::with_escalation_threshold(Duration::seconds(1));

        let alert_id = manager
            .send_alert(
                AlertLevel::Critical,
                AlertChannel::Email,
                "Critical",
                "Test",
                vec!["admin@example.com".to_string()],
            )
            .await
            .unwrap();

        // Wait for escalation threshold to pass
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let candidates = manager.get_escalation_candidates();
        assert!(candidates.iter().any(|a| a.alert_id == alert_id));
    }
}
