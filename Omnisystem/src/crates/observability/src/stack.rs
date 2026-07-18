//! Top-level observability stack tying metrics collection and SLA tracking
//! together behind a single, easy-to-use facade.

use crate::metrics::MetricsCollector;
use crate::sla::{SLACompliance, SLATarget, SLATracker};
use crate::tracing::init_tracing;

/// Owns a [`MetricsCollector`] and an [`SLATracker`] and coordinates them so
/// callers only need to talk to a single type.
pub struct ObservabilityStack {
    metrics: MetricsCollector,
    sla: SLATracker,
}

impl ObservabilityStack {
    /// Create a new stack tracking SLA compliance against `target`.
    pub fn new(target: SLATarget) -> Self {
        Self {
            metrics: MetricsCollector::new(),
            sla: SLATracker::new(target),
        }
    }

    /// Initialize the stack (sets up structured tracing).
    pub async fn initialize(&self) -> Result<(), String> {
        init_tracing()
    }

    /// Record an operation observation into both the metrics collector and
    /// the SLA tracker.
    pub fn record_operation(&self, operation: &str, latency_ms: f64, success: bool) {
        self.metrics.record(operation, latency_ms, success);
        self.sla.record(operation, latency_ms, success);
    }

    /// Get overall SLA compliance.
    pub fn get_sla_compliance(&self) -> SLACompliance {
        self.sla.get_compliance()
    }

    /// Get SLA compliance for a specific operation.
    pub fn get_operation_sla_compliance(&self, operation: &str) -> SLACompliance {
        self.sla.get_operation_compliance(operation)
    }

    /// Export collected metrics in Prometheus text format.
    pub async fn export_prometheus(&self) -> Result<String, String> {
        self.metrics.export_prometheus().await
    }

    /// Export collected metrics as JSON.
    pub fn export_json(&self) -> Result<String, String> {
        self.metrics.export_json()
    }

    /// Access the underlying metrics collector directly.
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    /// Access the underlying SLA tracker directly.
    pub fn sla_tracker(&self) -> &SLATracker {
        &self.sla
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_target() -> SLATarget {
        SLATarget {
            p95_latency_ms: 100.0,
            p99_latency_ms: 200.0,
            availability_percent: 99.95,
        }
    }

    #[tokio::test]
    async fn test_stack_initialize() {
        let stack = ObservabilityStack::new(test_target());
        assert!(stack.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_stack_record_and_export() {
        let stack = ObservabilityStack::new(test_target());
        stack.record_operation("op", 42.0, true);

        let compliance = stack.get_sla_compliance();
        assert!(compliance.compliance_percent >= 0.0);

        let prometheus = stack.export_prometheus().await;
        assert!(prometheus.is_ok());
        assert!(prometheus.unwrap().contains("operations_total"));
    }
}
