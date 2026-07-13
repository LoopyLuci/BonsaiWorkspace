use crate::{ConsumerLag, Throughput, QueueHealth, Alert, QueueMetrics, MonitoringError, MonitoringResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct QueueMonitor {
    lags: Arc<DashMap<Uuid, ConsumerLag>>,
    throughputs: Arc<DashMap<Uuid, Throughput>>,
    healths: Arc<DashMap<Uuid, QueueHealth>>,
    alerts: Arc<DashMap<Uuid, Alert>>,
    metrics: Arc<DashMap<Uuid, QueueMetrics>>,
}

impl QueueMonitor {
    pub fn new() -> Self {
        Self {
            lags: Arc::new(DashMap::new()),
            throughputs: Arc::new(DashMap::new()),
            healths: Arc::new(DashMap::new()),
            alerts: Arc::new(DashMap::new()),
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub async fn record_lag(&self, lag: &ConsumerLag) -> MonitoringResult<()> {
        self.lags.insert(lag.lag_id, lag.clone());
        Ok(())
    }

    pub async fn record_throughput(&self, throughput: &Throughput) -> MonitoringResult<()> {
        self.throughputs.insert(throughput.throughput_id, throughput.clone());
        Ok(())
    }

    pub async fn check_health(&self, health: &QueueHealth) -> MonitoringResult<()> {
        self.healths.insert(health.health_id, health.clone());
        Ok(())
    }

    pub async fn create_alert(&self, alert: &Alert) -> MonitoringResult<()> {
        self.alerts.insert(alert.alert_id, alert.clone());
        Ok(())
    }

    pub async fn record_metrics(&self, metrics: &QueueMetrics) -> MonitoringResult<()> {
        self.metrics.insert(metrics.metrics_id, metrics.clone());
        Ok(())
    }

    pub fn lag_count(&self) -> usize {
        self.lags.len()
    }

    pub fn alert_count(&self) -> usize {
        self.alerts.len()
    }
}

impl Default for QueueMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_lag() {
        let monitor = QueueMonitor::new();
        let lag = ConsumerLag {
            lag_id: Uuid::new_v4(),
            consumer_group: "analytics".to_string(),
            topic: "events".to_string(),
            partition: 0,
            lag_messages: 1000,
            lag_timestamp: Utc::now(),
        };

        monitor.record_lag(&lag).await.unwrap();
        assert_eq!(monitor.lag_count(), 1);
    }

    #[tokio::test]
    async fn test_record_throughput() {
        let monitor = QueueMonitor::new();
        let throughput = Throughput {
            throughput_id: Uuid::new_v4(),
            topic: "logs".to_string(),
            messages_per_second: 1000.0,
            bytes_per_second: 1024 * 1024,
            measured_at: Utc::now(),
        };

        monitor.record_throughput(&throughput).await.unwrap();
    }

    #[tokio::test]
    async fn test_check_health() {
        let monitor = QueueMonitor::new();
        let health = QueueHealth {
            health_id: Uuid::new_v4(),
            topic: "orders".to_string(),
            is_healthy: true,
            error_rate: 0.1,
            checked_at: Utc::now(),
        };

        monitor.check_health(&health).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_alert() {
        let monitor = QueueMonitor::new();
        let alert = Alert {
            alert_id: Uuid::new_v4(),
            alert_type: "high_lag".to_string(),
            severity: "warning".to_string(),
            message: "Consumer lag exceeded threshold".to_string(),
            triggered_at: Utc::now(),
        };

        monitor.create_alert(&alert).await.unwrap();
        assert_eq!(monitor.alert_count(), 1);
    }
}
