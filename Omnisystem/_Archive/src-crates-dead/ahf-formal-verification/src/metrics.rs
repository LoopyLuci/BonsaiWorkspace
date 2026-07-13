//! Metrics Collection and Monitoring
//!
//! This module provides comprehensive metrics collection for AHF components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// Metrics for AHF components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Total decisions made
    pub total_decisions: u64,
    /// Total acceptances
    pub total_accepts: u64,
    /// Total rejections
    pub total_rejects: u64,
    /// Total escalations
    pub total_escalations: u64,
    /// Average decision latency in ms
    pub avg_latency_ms: f64,
    /// Total errors
    pub total_errors: u64,
    /// Verification success rate
    pub verification_success_rate: f64,
}

/// Metrics collector
pub struct MetricsCollector {
    decisions: Arc<RwLock<u64>>,
    accepts: Arc<RwLock<u64>>,
    rejects: Arc<RwLock<u64>>,
    escalations: Arc<RwLock<u64>>,
    errors: Arc<RwLock<u64>>,
    latencies: Arc<RwLock<HashMap<String, f64>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        MetricsCollector {
            decisions: Arc::new(RwLock::new(0)),
            accepts: Arc::new(RwLock::new(0)),
            rejects: Arc::new(RwLock::new(0)),
            escalations: Arc::new(RwLock::new(0)),
            errors: Arc::new(RwLock::new(0)),
            latencies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a decision
    pub fn record_decision(&self, decision_type: &str) {
        *self.decisions.write().unwrap() += 1;

        match decision_type {
            "accept" => *self.accepts.write().unwrap() += 1,
            "reject" => *self.rejects.write().unwrap() += 1,
            "escalate" => *self.escalations.write().unwrap() += 1,
            _ => {}
        }
    }

    /// Record latency
    pub fn record_latency(&self, component: &str, ms: f64) {
        self.latencies.write().unwrap().insert(component.to_string(), ms);
    }

    /// Record an error
    pub fn record_error(&self) {
        *self.errors.write().unwrap() += 1;
    }

    /// Get snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let total = *self.decisions.read().unwrap();
        let accepts = *self.accepts.read().unwrap();
        let rejects = *self.rejects.read().unwrap();
        let escalations = *self.escalations.read().unwrap();
        let errors = *self.errors.read().unwrap();

        let latencies = self.latencies.read().unwrap();
        let avg_latency = if latencies.is_empty() {
            0.0
        } else {
            let sum: f64 = latencies.values().sum();
            sum / latencies.len() as f64
        };

        let verification_success_rate = if total == 0 {
            0.0
        } else {
            (accepts as f64 / total as f64).clamp(0.0, 1.0)
        };

        MetricsSnapshot {
            timestamp: Utc::now(),
            total_decisions: total,
            total_accepts: accepts,
            total_rejects: rejects,
            total_escalations: escalations,
            avg_latency_ms: avg_latency,
            total_errors: errors,
            verification_success_rate,
        }
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
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        collector.record_decision("accept");
        collector.record_decision("accept");
        collector.record_decision("reject");

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.total_decisions, 3);
        assert_eq!(snapshot.total_accepts, 2);
        assert_eq!(snapshot.total_rejects, 1);
    }

    #[test]
    fn test_latency_recording() {
        let collector = MetricsCollector::new();
        collector.record_latency("verifier", 25.0);
        collector.record_latency("bias_detector", 15.0);

        let snapshot = collector.snapshot();
        assert!(snapshot.avg_latency_ms > 0.0);
    }

    #[test]
    fn test_error_recording() {
        let collector = MetricsCollector::new();
        collector.record_error();
        collector.record_error();

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.total_errors, 2);
    }
}
