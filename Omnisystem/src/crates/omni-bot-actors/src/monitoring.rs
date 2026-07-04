//! MonitoringAgent Actor - Watches system health and triggers alerts
//!
//! Responsibilities:
//! - Monitor system metrics (CPU, memory, latency)
//! - Detect anomalies
//! - Generate health reports
//! - Trigger alerts for critical conditions
//! - Track alert history

use crate::actor::{Actor, ActorId, Snapshot};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Critical => "critical",
            Self::Unknown => "unknown",
        }
    }
}

/// System metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub memory_percent: f32,
    pub active_actors: u32,
    pub message_queue_depth: u32,
    pub avg_latency_ms: f32,
    pub error_rate: f32,
}

impl SystemMetrics {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            cpu_percent: 0.0,
            memory_mb: 0,
            memory_percent: 0.0,
            active_actors: 0,
            message_queue_depth: 0,
            avg_latency_ms: 0.0,
            error_rate: 0.0,
        }
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub metrics: SystemMetrics,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

impl HealthReport {
    pub fn new(metrics: SystemMetrics) -> Self {
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Determine status and identify issues
        let mut status = HealthStatus::Healthy;

        if metrics.cpu_percent > 90.0 {
            status = HealthStatus::Critical;
            issues.push("CPU usage critical (>90%)".to_string());
            recommendations.push("Consider scaling resources or reducing workload".to_string());
        } else if metrics.cpu_percent > 70.0 {
            status = HealthStatus::Degraded;
            issues.push("CPU usage high (>70%)".to_string());
            recommendations.push("Monitor CPU trends".to_string());
        }

        if metrics.memory_percent > 90.0 {
            status = HealthStatus::Critical;
            issues.push("Memory usage critical (>90%)".to_string());
            recommendations.push("Increase memory or investigate memory leaks".to_string());
        } else if metrics.memory_percent > 70.0 {
            status = HealthStatus::Degraded;
            issues.push("Memory usage high (>70%)".to_string());
            recommendations.push("Monitor memory trends".to_string());
        }

        if metrics.error_rate > 0.1 {
            // > 10%
            status = HealthStatus::Critical;
            issues.push("High error rate".to_string());
            recommendations.push("Review error logs and investigate root causes".to_string());
        } else if metrics.error_rate > 0.05 {
            // > 5%
            status = HealthStatus::Degraded;
            issues.push("Elevated error rate".to_string());
        }

        if metrics.avg_latency_ms > 5000.0 {
            status = HealthStatus::Critical;
            issues.push("Latency critical (>5000ms)".to_string());
            recommendations.push("Investigate slow operations and optimize".to_string());
        } else if metrics.avg_latency_ms > 1000.0 {
            status = HealthStatus::Degraded;
            issues.push("Latency elevated (>1000ms)".to_string());
        }

        Self {
            status,
            timestamp: Utc::now(),
            metrics,
            issues,
            recommendations,
        }
    }
}

/// Alert event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub triggered_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

impl Alert {
    pub fn new(
        severity: AlertSeverity,
        title: String,
        message: String,
        triggered_by: String,
    ) -> Self {
        Self {
            alert_id: format!("alert-{}", uuid::Uuid::new_v4()),
            severity,
            title,
            message,
            timestamp: Utc::now(),
            triggered_by,
        }
    }
}

/// Messages for MonitoringAgent
#[derive(Debug, Clone)]
pub enum MonitoringAgentMessage {
    /// Collect and analyze metrics
    CollectMetrics { metrics: SystemMetrics },
    /// Get latest health report
    GetHealthReport,
    /// Get recent alerts
    GetAlerts { limit: usize },
    /// Clear alert history
    ClearAlerts,
    /// Get metrics history
    GetMetricsHistory { limit: usize },
    /// Stop the actor
    Stop,
}

/// Monitoring metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonitoringMetrics {
    pub total_collections: u64,
    pub critical_alerts: u64,
    pub error_alerts: u64,
    pub warning_alerts: u64,
    pub avg_health_status_samples: u64,
}

/// MonitoringAgent actor
pub struct MonitoringAgent {
    id: ActorId,
    metrics: MonitoringMetrics,
    latest_health: Option<HealthReport>,
    alerts: VecDeque<Alert>,
    metrics_history: VecDeque<SystemMetrics>,
    max_history: usize,
}

impl MonitoringAgent {
    pub fn new() -> Self {
        Self {
            id: ActorId::new(),
            metrics: MonitoringMetrics::default(),
            latest_health: None,
            alerts: VecDeque::with_capacity(100),
            metrics_history: VecDeque::with_capacity(1000),
            max_history: 1000,
        }
    }

    fn collect_metrics(&mut self, sys_metrics: SystemMetrics) {
        let report = HealthReport::new(sys_metrics.clone());

        // Generate alerts if needed
        match report.status {
            HealthStatus::Critical => {
                for issue in &report.issues {
                    let alert = Alert::new(
                        AlertSeverity::Critical,
                        "System Critical".to_string(),
                        issue.clone(),
                        "MonitoringAgent".to_string(),
                    );
                    self.metrics.critical_alerts += 1;
                    self.alerts.push_back(alert);
                }
            }
            HealthStatus::Degraded => {
                for issue in &report.issues {
                    let alert = Alert::new(
                        AlertSeverity::Warning,
                        "System Degraded".to_string(),
                        issue.clone(),
                        "MonitoringAgent".to_string(),
                    );
                    self.metrics.warning_alerts += 1;
                    self.alerts.push_back(alert);
                }
            }
            _ => {}
        }

        // Maintain history
        if self.metrics_history.len() >= self.max_history {
            self.metrics_history.pop_front();
        }
        self.metrics_history.push_back(sys_metrics);

        // Keep alerts limited
        if self.alerts.len() > 1000 {
            while self.alerts.len() > 800 {
                self.alerts.pop_front();
            }
        }

        self.latest_health = Some(report);
        self.metrics.total_collections += 1;
    }

    #[allow(dead_code)]
    fn get_health_status(&self) -> Option<HealthStatus> {
        self.latest_health.as_ref().map(|h| h.status)
    }
}

impl Default for MonitoringAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Actor for MonitoringAgent {
    type Message = MonitoringAgentMessage;

    fn id(&self) -> ActorId {
        self.id
    }

    async fn handle(&mut self, msg: Self::Message) -> Result<bool, String> {
        match msg {
            MonitoringAgentMessage::CollectMetrics { metrics } => {
                log::info!(
                    "[MonitoringAgent] Collecting metrics: CPU={}%, MEM={}%",
                    metrics.cpu_percent,
                    metrics.memory_percent
                );
                self.collect_metrics(metrics);
                Ok(true)
            }
            MonitoringAgentMessage::GetHealthReport => {
                if let Some(report) = &self.latest_health {
                    log::info!(
                        "[MonitoringAgent] Health status: {}",
                        report.status.as_str()
                    );
                } else {
                    log::info!("[MonitoringAgent] No health report available yet");
                }
                Ok(true)
            }
            MonitoringAgentMessage::GetAlerts { limit } => {
                let count = std::cmp::min(limit, self.alerts.len());
                log::info!("[MonitoringAgent] Recent alerts: {} of {}", count, self.alerts.len());
                Ok(true)
            }
            MonitoringAgentMessage::ClearAlerts => {
                let count = self.alerts.len();
                self.alerts.clear();
                log::info!("[MonitoringAgent] Cleared {} alerts", count);
                Ok(true)
            }
            MonitoringAgentMessage::GetMetricsHistory { limit } => {
                let count = std::cmp::min(limit, self.metrics_history.len());
                log::info!(
                    "[MonitoringAgent] Metrics history: {} of {}",
                    count,
                    self.metrics_history.len()
                );
                Ok(true)
            }
            MonitoringAgentMessage::Stop => {
                log::info!("[MonitoringAgent] Stop signal received");
                Ok(false)
            }
        }
    }

    async fn snapshot(&self) -> Result<Snapshot, String> {
        let alerts: Vec<_> = self.alerts.iter().cloned().collect();
        let state = serde_json::json!({
            "metrics": self.metrics,
            "latest_health": self.latest_health,
            "alerts_count": self.alerts.len(),
            "metrics_history_count": self.metrics_history.len(),
            "recent_alerts": &alerts[..std::cmp::min(10, alerts.len())],
        });

        Ok(Snapshot::new(
            self.id,
            "MonitoringAgent".to_string(),
            state,
        ))
    }

    async fn restore(&mut self, _snapshot: Snapshot) -> Result<(), String> {
        log::info!("[MonitoringAgent] Restored from snapshot");
        Ok(())
    }

    fn actor_type(&self) -> &'static str {
        "MonitoringAgent"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_report_healthy() {
        let metrics = SystemMetrics {
            cpu_percent: 30.0,
            memory_percent: 40.0,
            error_rate: 0.01,
            avg_latency_ms: 100.0,
            ..SystemMetrics::new()
        };

        let report = HealthReport::new(metrics);
        assert_eq!(report.status, HealthStatus::Healthy);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn test_health_report_critical_cpu() {
        let metrics = SystemMetrics {
            cpu_percent: 95.0,
            memory_percent: 40.0,
            error_rate: 0.01,
            avg_latency_ms: 100.0,
            ..SystemMetrics::new()
        };

        let report = HealthReport::new(metrics);
        assert_eq!(report.status, HealthStatus::Critical);
        assert!(report
            .issues
            .iter()
            .any(|i| i.contains("CPU")));
    }

    #[test]
    fn test_health_report_degraded_memory() {
        let metrics = SystemMetrics {
            cpu_percent: 30.0,
            memory_percent: 75.0,
            error_rate: 0.01,
            avg_latency_ms: 100.0,
            ..SystemMetrics::new()
        };

        let report = HealthReport::new(metrics);
        assert_eq!(report.status, HealthStatus::Degraded);
    }

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(
            AlertSeverity::Critical,
            "Test Alert".to_string(),
            "This is a test".to_string(),
            "TestComponent".to_string(),
        );

        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert_eq!(alert.title, "Test Alert");
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Info < AlertSeverity::Warning);
        assert!(AlertSeverity::Warning < AlertSeverity::Error);
        assert!(AlertSeverity::Error < AlertSeverity::Critical);
    }
}
