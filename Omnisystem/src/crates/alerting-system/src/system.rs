use crate::{AlertRule, Alert, AlertStatus, AlertSeverity, IncidentRecord, IncidentStatus, NotificationRoute, AlertingError, AlertingResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct AlertingSystem {
    rules: Arc<DashMap<Uuid, AlertRule>>,
    alerts: Arc<DashMap<Uuid, Alert>>,
    incidents: Arc<DashMap<Uuid, IncidentRecord>>,
    routes: Arc<DashMap<Uuid, NotificationRoute>>,
}

impl AlertingSystem {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
            alerts: Arc::new(DashMap::new()),
            incidents: Arc::new(DashMap::new()),
            routes: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_rule(&self, rule: &AlertRule) -> AlertingResult<()> {
        self.rules.insert(rule.rule_id, rule.clone());
        Ok(())
    }

    pub async fn check_threshold(&self, metric_name: &str, metric_value: f64) -> AlertingResult<Option<Alert>> {
        for rule in self.rules.iter() {
            if rule.value().metric_name == metric_name && rule.value().enabled {
                let violated = match rule.value().comparison_op.as_str() {
                    ">" => metric_value > rule.value().threshold,
                    "<" => metric_value < rule.value().threshold,
                    ">=" => metric_value >= rule.value().threshold,
                    "<=" => metric_value <= rule.value().threshold,
                    _ => false,
                };

                if violated {
                    let alert = Alert {
                        alert_id: Uuid::new_v4(),
                        rule_id: rule.value().rule_id,
                        metric_value,
                        severity: rule.value().severity,
                        created_at: Utc::now(),
                        status: AlertStatus::Triggered,
                    };

                    self.alerts.insert(alert.alert_id, alert.clone());
                    return Ok(Some(alert));
                }
            }
        }

        Ok(None)
    }

    pub async fn create_incident(&self, alert_id: Uuid, severity: AlertSeverity) -> AlertingResult<IncidentRecord> {
        let incident = IncidentRecord {
            incident_id: Uuid::new_v4(),
            alert_id,
            severity,
            created_at: Utc::now(),
            resolved_at: None,
            status: IncidentStatus::Open,
        };

        self.incidents.insert(incident.incident_id, incident.clone());
        Ok(incident)
    }

    pub async fn acknowledge_alert(&self, alert_id: Uuid) -> AlertingResult<()> {
        if let Some(mut entry) = self.alerts.get_mut(&alert_id) {
            entry.status = AlertStatus::Acknowledged;
        }

        Ok(())
    }

    pub async fn resolve_incident(&self, incident_id: Uuid) -> AlertingResult<()> {
        if let Some(mut entry) = self.incidents.get_mut(&incident_id) {
            entry.status = IncidentStatus::Resolved;
            entry.resolved_at = Some(Utc::now());
        }

        Ok(())
    }

    pub async fn route_alert(&self, severity: AlertSeverity) -> AlertingResult<NotificationRoute> {
        for route in self.routes.iter() {
            if route.value().severity == severity {
                return Ok(route.value().clone());
            }
        }

        Err(AlertingError::RoutingFailed)
    }

    pub fn alert_count(&self) -> usize {
        self.alerts.len()
    }
}

impl Default for AlertingSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_rule() {
        let system = AlertingSystem::new();
        let rule = AlertRule {
            rule_id: Uuid::new_v4(),
            metric_name: "cpu_usage".to_string(),
            threshold: 80.0,
            comparison_op: ">".to_string(),
            severity: AlertSeverity::Critical,
            enabled: true,
        };

        system.add_rule(&rule).await.unwrap();
    }

    #[tokio::test]
    async fn test_check_threshold() {
        let system = AlertingSystem::new();
        let rule = AlertRule {
            rule_id: Uuid::new_v4(),
            metric_name: "memory".to_string(),
            threshold: 85.0,
            comparison_op: ">".to_string(),
            severity: AlertSeverity::Warning,
            enabled: true,
        };

        system.add_rule(&rule).await.unwrap();
        let alert = system.check_threshold("memory", 90.0).await.unwrap();
        assert!(alert.is_some());
    }

    #[tokio::test]
    async fn test_create_incident() {
        let system = AlertingSystem::new();
        let alert_id = Uuid::new_v4();
        let incident = system.create_incident(alert_id, AlertSeverity::Critical).await.unwrap();

        assert_eq!(incident.alert_id, alert_id);
        assert_eq!(incident.status, IncidentStatus::Open);
    }

    #[tokio::test]
    async fn test_acknowledge_alert() {
        let system = AlertingSystem::new();
        let rule = AlertRule {
            rule_id: Uuid::new_v4(),
            metric_name: "latency".to_string(),
            threshold: 500.0,
            comparison_op: ">".to_string(),
            severity: AlertSeverity::Warning,
            enabled: true,
        };

        system.add_rule(&rule).await.unwrap();
        let alert = system.check_threshold("latency", 600.0).await.unwrap().unwrap();
        system.acknowledge_alert(alert.alert_id).await.unwrap();
    }
}
