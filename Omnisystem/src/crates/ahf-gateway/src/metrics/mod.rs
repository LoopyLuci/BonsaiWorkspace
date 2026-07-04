//! Metrics and monitoring for the Anti-Hallucination Gateway

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::collections::HashMap;

/// Metrics snapshot for a specific time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Total hallucinations detected
    pub hallucination_count: u64,
    /// False rejections (accepted claims marked as hallucinations)
    pub false_rejection_count: u64,
    /// False positives (hallucinations that slipped through)
    pub false_positive_count: u64,
    /// Average pipeline latency (ms)
    pub avg_latency_ms: f64,
    /// Claims rejected due to bias
    pub bias_blocked_count: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Timestamp of snapshot
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Per-model metrics
    pub per_model_metrics: HashMap<String, ModelMetrics>,
}

/// Per-model metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub model_id: String,
    pub requests: u64,
    pub hallucinations: u64,
    pub avg_confidence: f64,
    pub avg_grounding_score: f64,
    pub bias_score_avg: f64,
}

/// Real-time metrics collector
pub struct AhfMetrics {
    hallucination_count: Arc<AtomicU64>,
    false_rejection_count: Arc<AtomicU64>,
    false_positive_count: Arc<AtomicU64>,
    bias_blocked_count: Arc<AtomicU64>,
    total_requests: Arc<AtomicU64>,
    total_latency_ms: Arc<AtomicU64>,
    per_model: Arc<DashMap<String, ModelMetricsAccumulator>>,
}

struct ModelMetricsAccumulator {
    requests: u64,
    hallucinations: u64,
    confidence_sum: f64,
    grounding_sum: f64,
    bias_sum: f64,
}

impl AhfMetrics {
    pub fn new() -> Self {
        Self {
            hallucination_count: Arc::new(AtomicU64::new(0)),
            false_rejection_count: Arc::new(AtomicU64::new(0)),
            false_positive_count: Arc::new(AtomicU64::new(0)),
            bias_blocked_count: Arc::new(AtomicU64::new(0)),
            total_requests: Arc::new(AtomicU64::new(0)),
            total_latency_ms: Arc::new(AtomicU64::new(0)),
            per_model: Arc::new(DashMap::new()),
        }
    }

    /// Record a hallucination detection
    pub fn record_hallucination(&self) {
        let _ = self.hallucination_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a false rejection
    pub fn record_false_rejection(&self) {
        let _ = self.false_rejection_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a false positive
    pub fn record_false_positive(&self) {
        let _ = self.false_positive_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a bias-blocked claim
    pub fn record_bias_block(&self) {
        let _ = self.bias_blocked_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a request and latency
    pub fn record_request(&self, latency_ms: u64) {
        let _ = self.total_requests.fetch_add(1, Ordering::Relaxed);
        let _ = self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
    }

    /// Record per-model metrics
    pub fn record_model_metrics(
        &self,
        model_id: &str,
        confidence: f64,
        grounding_score: f64,
        bias_score: f64,
        is_hallucination: bool,
    ) {
        let mut entry = self.per_model
            .entry(model_id.to_string())
            .or_insert_with(|| ModelMetricsAccumulator {
                requests: 0,
                hallucinations: 0,
                confidence_sum: 0.0,
                grounding_sum: 0.0,
                bias_sum: 0.0,
            });

        entry.requests += 1;
        if is_hallucination {
            entry.hallucinations += 1;
        }
        entry.confidence_sum += confidence;
        entry.grounding_sum += grounding_score;
        entry.bias_sum += bias_score;
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let total_latency = self.total_latency_ms.load(Ordering::Relaxed);
        let avg_latency_ms = if total_requests > 0 {
            total_latency as f64 / total_requests as f64
        } else {
            0.0
        };

        let mut per_model_metrics = HashMap::new();
        for entry in self.per_model.iter() {
            let model_id = entry.key().clone();
            let acc = entry.value();
            let _ = per_model_metrics.insert(
                model_id.clone(),
                ModelMetrics {
                    model_id,
                    requests: acc.requests,
                    hallucinations: acc.hallucinations,
                    avg_confidence: if acc.requests > 0 {
                        acc.confidence_sum / acc.requests as f64
                    } else {
                        0.0
                    },
                    avg_grounding_score: if acc.requests > 0 {
                        acc.grounding_sum / acc.requests as f64
                    } else {
                        0.0
                    },
                    bias_score_avg: if acc.requests > 0 {
                        acc.bias_sum / acc.requests as f64
                    } else {
                        0.0
                    },
                },
            );
        }

        MetricsSnapshot {
            hallucination_count: self.hallucination_count.load(Ordering::Relaxed),
            false_rejection_count: self.false_rejection_count.load(Ordering::Relaxed),
            false_positive_count: self.false_positive_count.load(Ordering::Relaxed),
            avg_latency_ms,
            bias_blocked_count: self.bias_blocked_count.load(Ordering::Relaxed),
            total_requests,
            timestamp: chrono::Utc::now(),
            per_model_metrics,
        }
    }

    /// Get false rejection rate
    pub fn false_rejection_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        let false_rejections = self.false_rejection_count.load(Ordering::Relaxed);
        if total > 0 {
            false_rejections as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Get false positive rate
    pub fn false_positive_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        let false_positives = self.false_positive_count.load(Ordering::Relaxed);
        if total > 0 {
            false_positives as f64 / total as f64
        } else {
            0.0
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.hallucination_count.store(0, Ordering::Relaxed);
        self.false_rejection_count.store(0, Ordering::Relaxed);
        self.false_positive_count.store(0, Ordering::Relaxed);
        self.bias_blocked_count.store(0, Ordering::Relaxed);
        self.total_requests.store(0, Ordering::Relaxed);
        self.total_latency_ms.store(0, Ordering::Relaxed);
        self.per_model.clear();
    }
}

impl Clone for AhfMetrics {
    fn clone(&self) -> Self {
        Self {
            hallucination_count: Arc::clone(&self.hallucination_count),
            false_rejection_count: Arc::clone(&self.false_rejection_count),
            false_positive_count: Arc::clone(&self.false_positive_count),
            bias_blocked_count: Arc::clone(&self.bias_blocked_count),
            total_requests: Arc::clone(&self.total_requests),
            total_latency_ms: Arc::clone(&self.total_latency_ms),
            per_model: Arc::clone(&self.per_model),
        }
    }
}

impl Default for AhfMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = AhfMetrics::new();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.hallucination_count, 0);
        assert_eq!(snapshot.total_requests, 0);
    }

    #[test]
    fn test_record_hallucination() {
        let metrics = AhfMetrics::new();
        metrics.record_hallucination();
        metrics.record_hallucination();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.hallucination_count, 2);
    }

    #[test]
    fn test_record_request_and_latency() {
        let metrics = AhfMetrics::new();
        metrics.record_request(10);
        metrics.record_request(20);
        metrics.record_request(30);
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_requests, 3);
        assert_eq!(snapshot.avg_latency_ms, 20.0);
    }

    #[test]
    fn test_per_model_metrics() {
        let metrics = AhfMetrics::new();
        metrics.record_model_metrics("gpt-4", 0.95, 0.85, 0.1, false);
        metrics.record_model_metrics("gpt-4", 0.90, 0.80, 0.2, true);

        let snapshot = metrics.snapshot();
        let model_metrics = snapshot.per_model_metrics.get("gpt-4").unwrap();
        assert_eq!(model_metrics.requests, 2);
        assert_eq!(model_metrics.hallucinations, 1);
        assert!((model_metrics.avg_confidence - 0.925).abs() < 0.01);
    }

    #[test]
    fn test_false_rejection_rate() {
        let metrics = AhfMetrics::new();
        metrics.record_request(10);
        metrics.record_request(10);
        metrics.record_request(10);
        metrics.record_false_rejection();

        let rate = metrics.false_rejection_rate();
        assert!((rate - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = AhfMetrics::new();
        metrics.record_hallucination();
        metrics.record_bias_block();
        metrics.record_request(50);

        let snapshot = metrics.snapshot();
        assert!(snapshot.hallucination_count > 0);

        metrics.reset();
        let snapshot2 = metrics.snapshot();
        assert_eq!(snapshot2.hallucination_count, 0);
        assert_eq!(snapshot2.bias_blocked_count, 0);
        assert_eq!(snapshot2.total_requests, 0);
    }

    #[test]
    fn test_metrics_clone() {
        let metrics = AhfMetrics::new();
        metrics.record_hallucination();

        let metrics2 = metrics.clone();
        let snapshot = metrics2.snapshot();
        assert_eq!(snapshot.hallucination_count, 1);
    }
}
