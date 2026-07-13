//! Anti-Hallucination Gateway Actor

use crate::error::GatewayResult;
use crate::messages::{VerifyOutput, AhfResult};
use crate::config::ConfigManager;
use crate::metrics::AhfMetrics;
use crate::audit::AuditLog;
use crate::pipeline::AhfPipeline;
use ahf_core::{Decision, Criticality};
use omni_bot_actors::{Actor, ActorId, Snapshot};
use serde_json::json;
use std::time::Instant;

/// The main Anti-Hallucination Gateway Actor
pub struct AhgActor {
    id: ActorId,
    pipeline: AhfPipeline,
    config_manager: ConfigManager,
    metrics: AhfMetrics,
    audit_log: AuditLog,
}

impl AhgActor {
    /// Create a new AHG actor
    pub async fn new(
        config: crate::config::AhfConfig,
    ) -> GatewayResult<Self> {
        config.validate()?;

        let config_manager = ConfigManager::new(config.clone());
        let pipeline = AhfPipeline::new(config);
        let metrics = AhfMetrics::new();
        let audit_log = AuditLog::new();

        Ok(Self {
            id: ActorId::new(),
            pipeline,
            config_manager,
            metrics,
            audit_log,
        })
    }

    /// Process a verification request
    async fn handle_verify_output(&mut self, msg: VerifyOutput) -> GatewayResult<AhfResult> {
        let start = Instant::now();
        let config = self.config_manager.get().await;

        // Execute pipeline
        let pipeline_result = self
            .pipeline
            .verify(&msg.output, &msg.model_id, msg.criticality)
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;

        // Record metrics
        self.metrics.record_request(latency_ms);
        self.metrics.record_model_metrics(
            &msg.model_id,
            pipeline_result.debug_info.model_confidence,
            pipeline_result.debug_info.grounding_score,
            pipeline_result.debug_info.bias_score,
            pipeline_result.decision.decision == Decision::Reject,
        );

        // Create audit entry
        let request_id = uuid::Uuid::new_v4();
        let audit_entry = crate::audit::AuditEntry::new(
            request_id,
            format!("{:?}", pipeline_result.decision.decision),
            pipeline_result.debug_info.grounding_score,
            pipeline_result.debug_info.verification_valid,
            pipeline_result.debug_info.model_confidence,
            pipeline_result.debug_info.bias_score,
            pipeline_result.decision.explanation.clone(),
            msg.model_id.clone(),
        );

        // Add to audit log
        let _ = self.audit_log.add(audit_entry).await;

        // Build result
        let decision = pipeline_result.decision;
        let mut result = AhfResult::new(decision.clone())
            .with_latency(latency_ms)
            .with_timeout(pipeline_result.timed_out);

        // Add fallback/escalation as needed
        match decision.decision {
            Decision::Reject => {
                result = result.with_fallback("Output rejected due to verification failure. Please review and try again.".to_string());
            }
            Decision::Escalate => {
                result = result.with_escalation(decision.explanation.clone());
            }
            Decision::Accept => {
                result = result.with_output(msg.output.clone());
            }
        }

        Ok(result)
    }

    /// Get current metrics snapshot
    pub fn get_metrics_snapshot(&self) -> crate::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get audit log entries
    pub async fn get_audit_entries(&self) -> Vec<crate::audit::AuditEntry> {
        self.audit_log.get_all().await
    }
}

#[async_trait::async_trait]
impl Actor for AhgActor {
    type Message = VerifyOutput;

    fn id(&self) -> ActorId {
        self.id
    }

    fn actor_type(&self) -> &'static str {
        "AhgActor"
    }

    async fn handle(&mut self, msg: Self::Message) -> Result<bool, String> {
        match self.handle_verify_output(msg).await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::error!("Error handling verification: {}", e);
                Ok(true) // Continue processing
            }
        }
    }

    async fn snapshot(&self) -> Result<Snapshot, String> {
        let state = json!({
            "actor_id": self.id.0.to_string(),
            "actor_type": self.actor_type(),
            "metrics": self.metrics.snapshot(),
        });

        Ok(Snapshot::new(
            self.id,
            self.actor_type().to_string(),
            state,
        ))
    }

    async fn restore(&mut self, _snapshot: Snapshot) -> Result<(), String> {
        // In production, would restore state from snapshot
        Ok(())
    }

    async fn on_stop(&mut self) {
        tracing::info!("AhgActor {} stopping", self.id.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AhfConfig;

    #[tokio::test]
    async fn test_ahg_actor_creation() {
        let config = AhfConfig::default();
        let actor = AhgActor::new(config).await;
        assert!(actor.is_ok());

        let actor = actor.unwrap();
        assert_eq!(actor.actor_type(), "AhgActor");
    }

    #[tokio::test]
    async fn test_ahg_actor_verify() {
        let config = AhfConfig::default();
        let mut actor = AhgActor::new(config).await.unwrap();

        let msg = VerifyOutput {
            output: "Paris is the capital of France.".to_string(),
            model_id: "gpt-4".to_string(),
            criticality: Criticality::Medium,
        };

        let result = actor.handle_verify_output(msg).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.latency_ms > 0);
    }

    #[tokio::test]
    async fn test_ahg_actor_metrics() {
        let config = AhfConfig::default();
        let mut actor = AhgActor::new(config).await.unwrap();

        let msg = VerifyOutput {
            output: "Test output.".to_string(),
            model_id: "gpt-4".to_string(),
            criticality: Criticality::Medium,
        };

        let _ = actor.handle_verify_output(msg).await;

        let metrics = actor.get_metrics_snapshot();
        assert_eq!(metrics.total_requests, 1);
    }

    #[tokio::test]
    async fn test_ahg_actor_audit_log() {
        let config = AhfConfig::default();
        let mut actor = AhgActor::new(config).await.unwrap();

        let msg = VerifyOutput {
            output: "Test output.".to_string(),
            model_id: "gpt-4".to_string(),
            criticality: Criticality::Medium,
        };

        let _ = actor.handle_verify_output(msg).await;

        let entries = actor.get_audit_entries().await;
        assert!(entries.len() > 0);
    }

    #[tokio::test]
    async fn test_ahg_actor_snapshot() {
        let config = AhfConfig::default();
        let actor = AhgActor::new(config).await.unwrap();

        let snapshot = actor.snapshot().await;
        assert!(snapshot.is_ok());

        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.actor_type, "AhgActor");
    }
}
