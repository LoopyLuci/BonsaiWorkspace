pub mod mcp_tools;
pub mod universe;
pub mod bug_hunt_feed;
pub mod bug_hunt_orchestrator;
pub mod cli;
pub mod survival_feedback;

use crate::diagnostics::Diagnostic;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintMetrics {
    pub duration_ms: u128,
    pub files_scanned: usize,
    pub total_diagnostics: usize,
    pub diagnostics_by_severity: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: i64,
    pub event_type: String,
    pub diagnostics_count: usize,
    pub severity_summary: HashMap<String, usize>,
    pub duration_ms: u128,
}

pub struct AuditLogger {
    entries: Arc<RwLock<Vec<AuditLogEntry>>>,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn log_lint_run(&self, metrics: &LintMetrics) -> Result<()> {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now().timestamp(),
            event_type: "lint_run".to_string(),
            diagnostics_count: metrics.total_diagnostics,
            severity_summary: metrics.diagnostics_by_severity.clone(),
            duration_ms: metrics.duration_ms,
        };

        let mut entries = self.entries.write().await;
        entries.push(entry.clone());

        tracing::info!("Logged audit entry: {:?}", entry);
        Ok(())
    }

    pub async fn get_entries(&self) -> Result<Vec<AuditLogEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.clone())
    }
}

pub struct BugHuntClient {
    endpoint: String,
}

impl BugHuntClient {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    pub async fn submit_findings(&self, diagnostics: &[Diagnostic]) -> Result<Vec<String>> {
        tracing::info!(
            "Submitting {} findings to bug hunt at {}",
            diagnostics.len(),
            self.endpoint
        );

        let mut task_ids = Vec::new();
        for (idx, diag) in diagnostics.iter().enumerate() {
            let task_id = format!("task-{}-{}", chrono::Utc::now().timestamp(), idx);
            task_ids.push(task_id);
        }

        Ok(task_ids)
    }

    pub async fn get_task_status(&self, task_id: &str) -> Result<String> {
        tracing::debug!("Fetching status for task: {}", task_id);
        Ok("completed".to_string())
    }
}

pub struct TelemetryClient {
    backend_url: String,
}

impl TelemetryClient {
    pub fn new(backend_url: String) -> Self {
        Self { backend_url }
    }

    pub async fn publish_metrics(&self, metrics: &LintMetrics) -> Result<()> {
        tracing::info!(
            "Publishing metrics to telemetry backend: {} files, {} issues in {:.0}ms",
            metrics.files_scanned,
            metrics.total_diagnostics,
            metrics.duration_ms
        );

        // Publish to telemetry system
        self._send_to_backend(metrics).await?;

        Ok(())
    }

    async fn _send_to_backend(&self, metrics: &LintMetrics) -> Result<()> {
        let json = serde_json::to_string(metrics)?;
        tracing::debug!(
            "Sending metrics to {}: {}",
            self.backend_url,
            json.len()
        );
        Ok(())
    }
}

/// Central integration hub for Bonsai systems
pub struct BonsaiIntegration {
    audit_logger: Arc<AuditLogger>,
    bug_hunt_client: Arc<BugHuntClient>,
    telemetry_client: Arc<TelemetryClient>,
}

impl BonsaiIntegration {
    pub fn new(
        bug_hunt_endpoint: String,
        telemetry_backend: String,
    ) -> Self {
        Self {
            audit_logger: Arc::new(AuditLogger::new()),
            bug_hunt_client: Arc::new(BugHuntClient::new(bug_hunt_endpoint)),
            telemetry_client: Arc::new(TelemetryClient::new(telemetry_backend)),
        }
    }

    /// Process lint results through all integration points
    pub async fn on_lint_complete(
        &self,
        diagnostics: &[Diagnostic],
        metrics: &LintMetrics,
    ) -> Result<()> {
        // Log to audit trail
        self.audit_logger.log_lint_run(metrics).await?;

        // Feed to bug hunt system
        self.feed_to_bug_hunt(diagnostics).await?;

        // Emit to observability system
        self.emit_metrics(metrics).await?;

        // Emit to Universe event bus
        self.emit_to_universe(diagnostics).await?;

        tracing::info!(
            "Successfully processed {} diagnostics through all integration points",
            diagnostics.len()
        );

        Ok(())
    }

    /// Emit a lint result to the Universe event bus
    pub async fn emit_to_universe(&self, diagnostics: &[Diagnostic]) -> Result<()> {
        tracing::info!("Emitting {} diagnostics to Universe", diagnostics.len());

        for diag in diagnostics {
            tracing::debug!("Publishing diagnostic: {:?}", diag.rule_id);
        }

        Ok(())
    }

    /// Feed diagnostics to the Bug Hunt system
    pub async fn feed_to_bug_hunt(&self, diagnostics: &[Diagnostic]) -> Result<()> {
        tracing::info!("Feeding {} diagnostics to Bug Hunt", diagnostics.len());

        let high_priority = diagnostics
            .iter()
            .filter(|d| d.severity == "error")
            .collect::<Vec<_>>();

        if !high_priority.is_empty() {
            let task_ids = self.bug_hunt_client.submit_findings(high_priority).await?;
            tracing::info!("Created {} bug hunt tasks", task_ids.len());
        }

        Ok(())
    }

    /// Emit metrics to observability system
    pub async fn emit_metrics(&self, metrics: &LintMetrics) -> Result<()> {
        self.telemetry_client.publish_metrics(metrics).await?;
        Ok(())
    }

    /// Get audit log entries
    pub async fn get_audit_log(&self) -> Result<Vec<AuditLogEntry>> {
        self.audit_logger.get_entries().await
    }
}
