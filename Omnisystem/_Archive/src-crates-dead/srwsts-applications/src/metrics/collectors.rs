//! Metrics collection and snapshotting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A point-in-time snapshot of all metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub memory_mb: u64,
    pub frame_time_ms: f64,
    pub response_time_ms: f64,
    pub cpu_percent: f64,
    pub active_tasks: u64,
}

impl MetricsSnapshot {
    /// Create a new metrics snapshot
    pub fn new(
        memory_mb: u64,
        frame_time_ms: f64,
        response_time_ms: f64,
        cpu_percent: f64,
        active_tasks: u64,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            memory_mb,
            frame_time_ms,
            response_time_ms,
            cpu_percent,
            active_tasks,
        }
    }
}

/// Metrics collector that periodically captures snapshots
pub struct MetricsCollector {
    snapshots: Arc<RwLock<Vec<MetricsSnapshot>>>,
    interval_ms: u64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(interval_ms: u64) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(Vec::new())),
            interval_ms,
        }
    }

    /// Start collecting metrics
    pub async fn start(&self) {
        let snapshots = self.snapshots.clone();
        let interval = self.interval_ms;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;

                // Capture current metrics (simplified for testing)
                let snapshot = MetricsSnapshot::new(
                    (rand::random::<u64>() % 2048) + 128,
                    rand::random::<f64>() * 20.0,
                    rand::random::<f64>() * 100.0,
                    rand::random::<f64>() * 100.0,
                    rand::random::<u64>() % 100,
                );

                let mut snap_guard = snapshots.write().await;
                snap_guard.push(snapshot);

                // Keep last 1000 snapshots
                if snap_guard.len() > 1000 {
                    snap_guard.remove(0);
                }
            }
        });
    }

    /// Get all snapshots
    pub async fn get_snapshots(&self) -> Vec<MetricsSnapshot> {
        self.snapshots.read().await.clone()
    }

    /// Get latest snapshot
    pub async fn get_latest(&self) -> Option<MetricsSnapshot> {
        self.snapshots.read().await.last().cloned()
    }

    /// Clear snapshots
    pub async fn clear(&self) {
        self.snapshots.write().await.clear();
    }

    /// Get number of snapshots
    pub async fn count(&self) -> usize {
        self.snapshots.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_snapshot_creation() {
        let snapshot = MetricsSnapshot::new(512, 10.0, 50.0, 25.0, 10);
        assert_eq!(snapshot.memory_mb, 512);
        assert_eq!(snapshot.frame_time_ms, 10.0);
    }

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new(100);
        assert_eq!(collector.count().await, 0);
    }

    #[tokio::test]
    async fn test_metrics_collector_snapshots() {
        let collector = MetricsCollector::new(10);
        collector.start().await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let count = collector.count().await;
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_get_latest_snapshot() {
        let collector = MetricsCollector::new(10);
        collector.start().await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let latest = collector.get_latest().await;
        assert!(latest.is_some());
    }

    #[tokio::test]
    async fn test_clear_snapshots() {
        let collector = MetricsCollector::new(10);
        collector.start().await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        collector.clear().await;

        assert_eq!(collector.count().await, 0);
    }
}
