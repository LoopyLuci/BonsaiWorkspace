use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: DateTime<Utc>,
    pub crashes_found: u64,
    pub tests_executed: u64,
    pub coverage_percent: f64,
    pub avg_response_time_ms: f64,
}

pub struct MetricsCollector {
    crashes_found: AtomicU64,
    tests_executed: AtomicU64,
    total_response_time_ms: AtomicU64,
    test_count: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            crashes_found: AtomicU64::new(0),
            tests_executed: AtomicU64::new(0),
            total_response_time_ms: AtomicU64::new(0),
            test_count: AtomicU64::new(0),
        }
    }

    pub fn record_crash(&self) {
        self.crashes_found.fetch_add(1, Ordering::SeqCst);
    }

    pub fn record_test_execution(&self) {
        self.tests_executed.fetch_add(1, Ordering::SeqCst);
    }

    pub fn record_response_time(&self, millis: u64) {
        self.total_response_time_ms.fetch_add(millis, Ordering::SeqCst);
        self.test_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_snapshot(&self) -> MetricsSnapshot {
        let crashes = self.crashes_found.load(Ordering::SeqCst);
        let tests = self.tests_executed.load(Ordering::SeqCst);
        let total_time = self.total_response_time_ms.load(Ordering::SeqCst);
        let test_count = self.test_count.load(Ordering::SeqCst);

        let avg_time = if test_count > 0 {
            total_time as f64 / test_count as f64
        } else {
            0.0
        };

        MetricsSnapshot {
            timestamp: Utc::now(),
            crashes_found: crashes,
            tests_executed: tests,
            coverage_percent: if tests > 0 {
                (crashes as f64 / tests as f64) * 100.0
            } else {
                0.0
            },
            avg_response_time_ms: avg_time,
        }
    }

    pub fn reset(&self) {
        self.crashes_found.store(0, Ordering::SeqCst);
        self.tests_executed.store(0, Ordering::SeqCst);
        self.total_response_time_ms.store(0, Ordering::SeqCst);
        self.test_count.store(0, Ordering::SeqCst);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = MetricsCollector::new();
        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.crashes_found, 0);
        assert_eq!(snapshot.tests_executed, 0);
    }

    #[test]
    fn test_record_crash() {
        let metrics = MetricsCollector::new();
        metrics.record_crash();
        metrics.record_crash();
        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.crashes_found, 2);
    }

    #[test]
    fn test_record_test_execution() {
        let metrics = MetricsCollector::new();
        for _ in 0..10 {
            metrics.record_test_execution();
        }
        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.tests_executed, 10);
    }

    #[test]
    fn test_response_time_tracking() {
        let metrics = MetricsCollector::new();
        metrics.record_response_time(100);
        metrics.record_response_time(200);
        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.avg_response_time_ms, 150.0);
    }

    #[test]
    fn test_coverage_calculation() {
        let metrics = MetricsCollector::new();
        for _ in 0..100 {
            metrics.record_test_execution();
        }
        for _ in 0..50 {
            metrics.record_crash();
        }
        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.coverage_percent, 50.0);
    }

    #[test]
    fn test_reset() {
        let metrics = MetricsCollector::new();
        metrics.record_crash();
        metrics.record_test_execution();
        assert_eq!(metrics.get_snapshot().crashes_found, 1);

        metrics.reset();
        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.crashes_found, 0);
        assert_eq!(snapshot.tests_executed, 0);
    }
}
