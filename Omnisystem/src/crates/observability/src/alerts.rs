use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub metric: String,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub comparison: AlertComparison,
}

/// Comparison operator for alert rules
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AlertComparison {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
}

/// Fired alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub metric: String,
    pub value: f64,
    pub threshold: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Alert engine for evaluating rules
pub struct AlertEngine {
    rules: Arc<RwLock<Vec<AlertRule>>>,
    active_alerts: Arc<RwLock<Vec<Alert>>>,
}

impl AlertEngine {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            active_alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add an alert rule
    pub fn add_rule(&self, rule: AlertRule) {
        self.rules.write().push(rule);
        tracing::info!("Alert rule added: {}", rule.name);
    }

    /// Check all rules against a metric value
    pub fn check_rules(&self, metric: &str, value: f64) -> Option<Alert> {
        let rules = self.rules.read();

        for rule in rules.iter() {
            if rule.metric == metric {
                if self.should_alert(&rule, value) {
                    let alert = Alert {
                        rule_name: rule.name.clone(),
                        severity: rule.severity,
                        metric: rule.metric.clone(),
                        value,
                        threshold: rule.threshold,
                        timestamp: chrono::Utc::now(),
                    };

                    self.active_alerts.write().push(alert.clone());
                    tracing::warn!("Alert fired: {} (value={}, threshold={})",
                        rule.name, value, rule.threshold);

                    return Some(alert);
                }
            }
        }

        None
    }

    /// Evaluate if alert should fire
    fn should_alert(&self, rule: &AlertRule, value: f64) -> bool {
        match rule.comparison {
            AlertComparison::GreaterThan => value > rule.threshold,
            AlertComparison::LessThan => value < rule.threshold,
            AlertComparison::Equal => (value - rule.threshold).abs() < f64::EPSILON,
            AlertComparison::NotEqual => (value - rule.threshold).abs() >= f64::EPSILON,
        }
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().clone()
    }

    /// Clear resolved alerts
    pub fn clear_resolved(&self, rule_name: &str) {
        let mut alerts = self.active_alerts.write();
        alerts.retain(|a| a.rule_name != rule_name);
    }

    /// Get all rules
    pub fn get_rules(&self) -> Vec<AlertRule> {
        self.rules.read().clone()
    }
}

impl Default for AlertEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_rule() {
        let engine = AlertEngine::new();
        let rule = AlertRule {
            name: "high_latency".to_string(),
            metric: "latency_ms".to_string(),
            threshold: 100.0,
            severity: AlertSeverity::Error,
            comparison: AlertComparison::GreaterThan,
        };

        engine.add_rule(rule);
        assert_eq!(engine.get_rules().len(), 1);

        // Should trigger alert
        let alert = engine.check_rules("latency_ms", 150.0);
        assert!(alert.is_some());

        // Should not trigger alert
        let alert = engine.check_rules("latency_ms", 50.0);
        assert!(alert.is_none());
    }
}
