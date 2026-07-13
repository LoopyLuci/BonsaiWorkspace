//! Metrics and observability for the Anti-Hallucination Framework
//!
//! Tracks performance, decision quality, and system health metrics.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Performance metrics for the AHF system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AhfMetrics {
    /// Total hallucinations detected
    pub hallucination_count: u64,
    /// False rejections (incorrectly flagged as hallucinations)
    pub false_rejection_count: u64,
    /// Average verification latency in milliseconds
    pub avg_latency_ms: f64,
    /// Total claims verified
    pub total_verified: u64,
    /// Number of bias violations detected
    pub bias_blocks_count: u64,
    /// Number of escalations to human review
    pub escalation_count: u64,
    /// Number of accepted decisions
    pub decisions_accepted: u64,
    /// Number of rejected decisions
    pub decisions_rejected: u64,
    /// Timestamp of last metric update
    pub last_updated: DateTime<Utc>,
}

impl AhfMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            hallucination_count: 0,
            false_rejection_count: 0,
            avg_latency_ms: 0.0,
            total_verified: 0,
            bias_blocks_count: 0,
            escalation_count: 0,
            decisions_accepted: 0,
            decisions_rejected: 0,
            last_updated: Utc::now(),
        }
    }

    /// Record a hallucination detection
    pub fn record_hallucination(&mut self) {
        self.hallucination_count += 1;
        self.last_updated = Utc::now();
    }

    /// Record a false rejection (incorrectly flagged)
    pub fn record_false_rejection(&mut self) {
        self.false_rejection_count += 1;
        self.last_updated = Utc::now();
    }

    /// Record a verified claim
    pub fn record_verification(&mut self, latency_ms: f64) {
        self.total_verified += 1;
        // Update running average
        let prev_total = self.total_verified - 1;
        if prev_total > 0 {
            let prev_sum = self.avg_latency_ms * prev_total as f64;
            self.avg_latency_ms = (prev_sum + latency_ms) / self.total_verified as f64;
        } else {
            self.avg_latency_ms = latency_ms;
        }
        self.last_updated = Utc::now();
    }

    /// Record a bias violation
    pub fn record_bias_block(&mut self) {
        self.bias_blocks_count += 1;
        self.last_updated = Utc::now();
    }

    /// Record an escalation
    pub fn record_escalation(&mut self) {
        self.escalation_count += 1;
        self.last_updated = Utc::now();
    }

    /// Record an accept decision
    pub fn record_accept(&mut self) {
        self.decisions_accepted += 1;
        self.last_updated = Utc::now();
    }

    /// Record a reject decision
    pub fn record_reject(&mut self) {
        self.decisions_rejected += 1;
        self.last_updated = Utc::now();
    }

    /// Calculate false positive rate
    pub fn false_positive_rate(&self) -> f64 {
        if self.total_verified == 0 {
            0.0
        } else {
            self.false_rejection_count as f64 / self.total_verified as f64
        }
    }

    /// Calculate true positive rate (recall)
    pub fn detection_rate(&self) -> f64 {
        if self.hallucination_count + self.false_rejection_count == 0 {
            0.0
        } else {
            self.hallucination_count as f64
                / (self.hallucination_count + self.false_rejection_count) as f64
        }
    }

    /// Calculate escalation rate
    pub fn escalation_rate(&self) -> f64 {
        if self.total_verified == 0 {
            0.0
        } else {
            self.escalation_count as f64 / self.total_verified as f64
        }
    }

    /// Calculate acceptance rate
    pub fn acceptance_rate(&self) -> f64 {
        let total = self.decisions_accepted + self.decisions_rejected;
        if total == 0 {
            0.0
        } else {
            self.decisions_accepted as f64 / total as f64
        }
    }
}

impl Default for AhfMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Atomic metrics for thread-safe increments without cloning
#[derive(Debug, Clone)]
pub struct AtomicAhfMetrics {
    hallucination_count: Arc<AtomicU64>,
    false_rejection_count: Arc<AtomicU64>,
    total_verified: Arc<AtomicU64>,
    bias_blocks_count: Arc<AtomicU64>,
    escalation_count: Arc<AtomicU64>,
}

impl AtomicAhfMetrics {
    /// Create new atomic metrics
    pub fn new() -> Self {
        Self {
            hallucination_count: Arc::new(AtomicU64::new(0)),
            false_rejection_count: Arc::new(AtomicU64::new(0)),
            total_verified: Arc::new(AtomicU64::new(0)),
            bias_blocks_count: Arc::new(AtomicU64::new(0)),
            escalation_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increment hallucination count
    pub fn inc_hallucination(&self) {
        let _ = self.hallucination_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment false rejection count
    pub fn inc_false_rejection(&self) {
        let _ = self.false_rejection_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment verified count
    pub fn inc_verified(&self) {
        let _ = self.total_verified.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment bias blocks count
    pub fn inc_bias_blocks(&self) {
        let _ = self.bias_blocks_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment escalation count
    pub fn inc_escalation(&self) {
        let _ = self.escalation_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics
    pub fn snapshot(&self) -> AhfMetrics {
        AhfMetrics {
            hallucination_count: self.hallucination_count.load(Ordering::Relaxed),
            false_rejection_count: self.false_rejection_count.load(Ordering::Relaxed),
            avg_latency_ms: 0.0, // Not tracked in atomic version
            total_verified: self.total_verified.load(Ordering::Relaxed),
            bias_blocks_count: self.bias_blocks_count.load(Ordering::Relaxed),
            escalation_count: self.escalation_count.load(Ordering::Relaxed),
            decisions_accepted: 0,
            decisions_rejected: 0,
            last_updated: Utc::now(),
        }
    }

    /// Reset all counters
    pub fn reset(&self) {
        self.hallucination_count.store(0, Ordering::Relaxed);
        self.false_rejection_count.store(0, Ordering::Relaxed);
        self.total_verified.store(0, Ordering::Relaxed);
        self.bias_blocks_count.store(0, Ordering::Relaxed);
        self.escalation_count.store(0, Ordering::Relaxed);
    }
}

impl Default for AtomicAhfMetrics {
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
        assert_eq!(metrics.hallucination_count, 0);
        assert_eq!(metrics.total_verified, 0);
    }

    #[test]
    fn test_record_hallucination() {
        let mut metrics = AhfMetrics::new();
        metrics.record_hallucination();
        assert_eq!(metrics.hallucination_count, 1);
    }

    #[test]
    fn test_record_verification() {
        let mut metrics = AhfMetrics::new();
        metrics.record_verification(50.0);
        metrics.record_verification(100.0);
        assert_eq!(metrics.total_verified, 2);
        assert!((metrics.avg_latency_ms - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_false_positive_rate() {
        let mut metrics = AhfMetrics::new();
        metrics.total_verified = 100;
        metrics.false_rejection_count = 5;
        assert!((metrics.false_positive_rate() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_detection_rate() {
        let mut metrics = AhfMetrics::new();
        metrics.hallucination_count = 80;
        metrics.false_rejection_count = 20;
        assert!((metrics.detection_rate() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_escalation_rate() {
        let mut metrics = AhfMetrics::new();
        metrics.total_verified = 100;
        metrics.escalation_count = 10;
        assert!((metrics.escalation_rate() - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_acceptance_rate() {
        let mut metrics = AhfMetrics::new();
        metrics.decisions_accepted = 75;
        metrics.decisions_rejected = 25;
        assert!((metrics.acceptance_rate() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_atomic_metrics_increments() {
        let metrics = AtomicAhfMetrics::new();
        metrics.inc_hallucination();
        metrics.inc_hallucination();
        metrics.inc_verified();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.hallucination_count, 2);
        assert_eq!(snapshot.total_verified, 1);
    }

    #[test]
    fn test_atomic_metrics_reset() {
        let metrics = AtomicAhfMetrics::new();
        metrics.inc_hallucination();
        metrics.inc_verified();
        metrics.inc_bias_blocks();

        metrics.reset();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.hallucination_count, 0);
        assert_eq!(snapshot.total_verified, 0);
        assert_eq!(snapshot.bias_blocks_count, 0);
    }

    #[test]
    fn test_metrics_serialization() {
        let mut metrics = AhfMetrics::new();
        metrics.record_hallucination();
        metrics.record_verification(42.5);

        let json = serde_json::to_string(&metrics).unwrap();
        let deserialized: AhfMetrics = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.hallucination_count, 1);
        assert_eq!(deserialized.total_verified, 1);
    }
}
