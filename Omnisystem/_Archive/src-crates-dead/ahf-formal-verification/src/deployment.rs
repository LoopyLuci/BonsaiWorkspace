//! Production Deployment Infrastructure
//!
//! This module provides health checks, graceful shutdown, metrics export, and
//! audit log archival for production AHF deployment.

use crate::error::{VerificationError, VerificationResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use std::sync::RwLock;

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Service name
    pub service_name: String,
    /// Service version
    pub version: String,
    /// Environment (dev, staging, production)
    pub environment: String,
    /// Port to listen on
    pub port: u16,
    /// Enable health checks
    pub health_checks_enabled: bool,
    /// Health check interval in seconds
    pub health_check_interval: u64,
    /// Metrics export enabled
    pub metrics_enabled: bool,
    /// Audit logging enabled
    pub audit_logging_enabled: bool,
    /// Graceful shutdown timeout in seconds
    pub shutdown_timeout: u64,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        DeploymentConfig {
            service_name: "ahf-service".to_string(),
            version: "1.0.0".to_string(),
            environment: "production".to_string(),
            port: 8080,
            health_checks_enabled: true,
            health_check_interval: 30,
            metrics_enabled: true,
            audit_logging_enabled: true,
            shutdown_timeout: 30,
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Is service healthy?
    pub healthy: bool,
    /// Service status
    pub status: String,
    /// Components health
    pub components: Vec<ComponentHealth>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Health status of a single component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Is component healthy?
    pub healthy: bool,
    /// Status message
    pub message: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Health checker
pub struct HealthChecker {
    config: DeploymentConfig,
    start_time: Utc,
    component_checks: Arc<RwLock<Vec<ComponentCheckFn>>>,
}

type ComponentCheckFn = Box<dyn Fn() -> VerificationResult<ComponentHealth> + Send + Sync>;

impl HealthChecker {
    /// Create a new health checker
    pub fn new(config: DeploymentConfig) -> Self {
        HealthChecker {
            config,
            start_time: chrono::Utc::now(),
            component_checks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a component health check
    pub fn register_check(
        &self,
        check: Box<dyn Fn() -> VerificationResult<ComponentHealth> + Send + Sync>,
    ) {
        self.component_checks.write().push(check);
    }

    /// Perform health check
    pub fn check(&self) -> HealthCheckResult {
        let mut components = Vec::new();

        // Run all registered checks
        for check in self.component_checks.read().iter() {
            match check() {
                Ok(health) => components.push(health),
                Err(e) => {
                    components.push(ComponentHealth {
                        name: "unknown".to_string(),
                        healthy: false,
                        message: e.to_string(),
                        response_time_ms: 0,
                    });
                }
            }
        }

        let overall_healthy = components.iter().all(|c| c.healthy);
        let now = chrono::Utc::now();
        let uptime = (now - self.start_time).num_seconds() as u64;

        HealthCheckResult {
            healthy: overall_healthy,
            status: if overall_healthy {
                "OK".to_string()
            } else {
                "UNHEALTHY".to_string()
            },
            components,
            timestamp: now,
            uptime_seconds: uptime,
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique identifier
    pub id: Uuid,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: String,
    /// Event details
    pub details: serde_json::Value,
    /// User/actor
    pub actor: String,
    /// Severity level
    pub severity: String,
}

/// Audit logger
pub struct AuditLogger {
    entries: Arc<RwLock<Vec<AuditLogEntry>>>,
    max_entries: usize,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(max_entries: usize) -> Self {
        AuditLogger {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
        }
    }

    /// Log an event
    pub fn log_event(
        &self,
        event_type: impl Into<String>,
        details: serde_json::Value,
        actor: impl Into<String>,
        severity: impl Into<String>,
    ) -> VerificationResult<()> {
        let entry = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: event_type.into(),
            details,
            actor: actor.into(),
            severity: severity.into(),
        };

        let mut entries = self.entries.write();
        entries.push(entry);

        // Keep only recent entries
        if entries.len() > self.max_entries {
            entries.remove(0);
        }

        Ok(())
    }

    /// Get recent audit logs
    pub fn get_recent(&self, count: usize) -> Vec<AuditLogEntry> {
        self.entries
            .read()
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get total log entries
    pub fn total_entries(&self) -> usize {
        self.entries.read().len()
    }

    /// Archive logs to a file
    pub fn archive_to_file(&self, path: &str) -> VerificationResult<()> {
        let entries = self.entries.read();
        let json = serde_json::to_string_pretty(&*entries)
            .map_err(|e| VerificationError::SerializationError(e))?;
        std::fs::write(path, json).map_err(|e| VerificationError::IoError(e))?;
        Ok(())
    }
}

/// Prometheus metrics exporter
pub struct PrometheusExporter {
    registry: Registry,
    /// Counter for decisions made
    decisions_total: Counter,
    /// Gauge for current processing time
    processing_time_ms: Gauge,
    /// Histogram for decision latencies
    decision_latency_histogram: Histogram,
    /// Counter for errors
    errors_total: Counter,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter
    pub fn new() -> VerificationResult<Self> {
        let registry = Registry::new();

        let decisions_total = Counter::new("ahf_decisions_total", "Total decisions made")
            .map_err(|e| VerificationError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(decisions_total.clone()))
            .map_err(|e| VerificationError::MetricsError(e.to_string()))?;

        let processing_time_ms = Gauge::new("ahf_processing_time_ms", "Current processing time")
            .map_err(|e| VerificationError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(processing_time_ms.clone()))
            .map_err(|e| VerificationError::MetricsError(e.to_string()))?;

        let decision_latency_histogram = Histogram::new("ahf_decision_latency_ms", "Decision latency")
            .map_err(|e| VerificationError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(decision_latency_histogram.clone()))
            .map_err(|e| VerificationError::MetricsError(e.to_string()))?;

        let errors_total = Counter::new("ahf_errors_total", "Total errors")
            .map_err(|e| VerificationError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(errors_total.clone()))
            .map_err(|e| VerificationError::MetricsError(e.to_string()))?;

        Ok(PrometheusExporter {
            registry,
            decisions_total,
            processing_time_ms,
            decision_latency_histogram,
            errors_total,
        })
    }

    /// Record a decision
    pub fn record_decision(&self, latency_ms: f64) {
        self.decisions_total.inc();
        self.decision_latency_histogram.observe(latency_ms);
    }

    /// Record an error
    pub fn record_error(&self) {
        self.errors_total.inc();
    }

    /// Set processing time
    pub fn set_processing_time(&self, ms: f64) {
        self.processing_time_ms.set(ms);
    }

    /// Export metrics in Prometheus text format
    pub fn export_metrics(&self) -> VerificationResult<String> {
        use prometheus::TextEncoder;
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder
            .encode(&metric_families, &mut Vec::new())
            .map_err(|e| VerificationError::MetricsError(e.to_string()))
            .map(|_| {
                let mut output = Vec::new();
                let _ = encoder.encode(&metric_families, &mut output);
                String::from_utf8_lossy(&output).to_string()
            })
    }
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new().expect("Failed to create default PrometheusExporter")
    }
}

/// Graceful shutdown handler
pub struct GracefulShutdownHandler {
    config: DeploymentConfig,
    in_shutdown: Arc<RwLock<bool>>,
}

impl GracefulShutdownHandler {
    /// Create a new graceful shutdown handler
    pub fn new(config: DeploymentConfig) -> Self {
        GracefulShutdownHandler {
            config,
            in_shutdown: Arc::new(RwLock::new(false)),
        }
    }

    /// Signal shutdown
    pub fn signal_shutdown(&self) {
        *self.in_shutdown.write() = true;
        tracing::info!(
            "Graceful shutdown initiated with timeout: {}s",
            self.config.shutdown_timeout
        );
    }

    /// Check if shutdown is in progress
    pub fn is_shutting_down(&self) -> bool {
        *self.in_shutdown.read()
    }

    /// Wait for shutdown timeout
    pub async fn wait_for_shutdown(&self) {
        let duration = std::time::Duration::from_secs(self.config.shutdown_timeout);
        tokio::time::sleep(duration).await;
        tracing::info!("Graceful shutdown timeout reached");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_config_defaults() {
        let config = DeploymentConfig::default();
        assert_eq!(config.service_name, "ahf-service");
        assert_eq!(config.port, 8080);
        assert!(config.health_checks_enabled);
    }

    #[test]
    fn test_health_checker_creation() {
        let config = DeploymentConfig::default();
        let checker = HealthChecker::new(config);
        let result = checker.check();
        assert!(result.components.is_empty());
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::new(100);

        let details = serde_json::json!({
            "action": "decision_made",
            "decision": "ACCEPT"
        });

        assert!(logger.log_event(
            "DECISION",
            details,
            "system",
            "INFO"
        ).is_ok());

        let recent = logger.get_recent(10);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].event_type, "DECISION");
    }

    #[test]
    fn test_audit_logger_max_entries() {
        let logger = AuditLogger::new(2);

        for i in 0..5 {
            let details = serde_json::json!({"entry": i});
            let _ = logger.log_event("TEST", details, "system", "INFO");
        }

        assert_eq!(logger.total_entries(), 2);
    }

    #[test]
    fn test_prometheus_exporter() {
        let exporter = PrometheusExporter::new().unwrap();
        exporter.record_decision(50.0);
        exporter.record_decision(75.0);
        exporter.set_processing_time(60.0);

        // Just verify it doesn't panic and exports something
        let metrics = exporter.export_metrics();
        assert!(metrics.is_ok());
    }

    #[test]
    fn test_graceful_shutdown_handler() {
        let config = DeploymentConfig::default();
        let handler = GracefulShutdownHandler::new(config);

        assert!(!handler.is_shutting_down());
        handler.signal_shutdown();
        assert!(handler.is_shutting_down());
    }
}
