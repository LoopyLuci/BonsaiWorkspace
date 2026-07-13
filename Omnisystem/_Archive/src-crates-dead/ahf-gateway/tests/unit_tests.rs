//! Unit tests for AHG components

use ahf_gateway::{
    AhfConfig, AhfMetrics, AuditLog, AuditEntry, ConfigManager,
};
use uuid::Uuid;

#[test]
fn test_config_default() {
    let cfg = AhfConfig::default();
    assert_eq!(cfg.grounding_threshold, 0.7);
    assert_eq!(cfg.confidence_threshold, 0.6);
    assert_eq!(cfg.bias_threshold, 0.5);
    assert_eq!(cfg.pipeline_timeout_ms, 50);
}

#[test]
fn test_config_validation_pass() {
    let cfg = AhfConfig::default();
    assert!(cfg.validate().is_ok());
}

#[test]
fn test_config_validation_grounding_threshold() {
    let mut cfg = AhfConfig::default();
    cfg.grounding_threshold = 1.5;
    assert!(cfg.validate().is_err());

    cfg.grounding_threshold = -0.5;
    assert!(cfg.validate().is_err());

    cfg.grounding_threshold = 0.7;
    assert!(cfg.validate().is_ok());
}

#[test]
fn test_config_validation_pipeline_timeout() {
    let mut cfg = AhfConfig::default();
    cfg.pipeline_timeout_ms = 0;
    assert!(cfg.validate().is_err());

    cfg.pipeline_timeout_ms = 50;
    assert!(cfg.validate().is_ok());
}

#[test]
fn test_config_metadata() {
    let mut cfg = AhfConfig::default();
    cfg.set_metadata("key1".to_string(), "value1".to_string());

    assert_eq!(cfg.get_metadata("key1"), Some("value1"));
    assert_eq!(cfg.get_metadata("nonexistent"), None);
}

#[test]
fn test_config_version_update() {
    let mut cfg = AhfConfig::default();
    assert_eq!(cfg.version.version, 1);

    cfg.update_version("Updated".to_string());
    assert_eq!(cfg.version.version, 2);
    assert_eq!(cfg.version.description, "Updated");
}

#[test]
fn test_metrics_creation() {
    let metrics = AhfMetrics::new();
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.hallucination_count, 0);
    assert_eq!(snapshot.total_requests, 0);
    assert_eq!(snapshot.bias_blocked_count, 0);
}

#[test]
fn test_metrics_hallucination_count() {
    let metrics = AhfMetrics::new();

    metrics.record_hallucination();
    metrics.record_hallucination();
    metrics.record_hallucination();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.hallucination_count, 3);
}

#[test]
fn test_metrics_request_and_latency() {
    let metrics = AhfMetrics::new();

    metrics.record_request(10);
    metrics.record_request(20);
    metrics.record_request(30);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_requests, 3);
    assert_eq!(snapshot.avg_latency_ms, 20.0);
}

#[test]
fn test_metrics_bias_blocks() {
    let metrics = AhfMetrics::new();

    metrics.record_bias_block();
    metrics.record_bias_block();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.bias_blocked_count, 2);
}

#[test]
fn test_metrics_false_rejection_rate() {
    let metrics = AhfMetrics::new();

    metrics.record_request(10);
    metrics.record_request(10);
    metrics.record_request(10);
    metrics.record_false_rejection();

    let rate = metrics.false_rejection_rate();
    assert!((rate - 1.0/3.0).abs() < 0.001);
}

#[test]
fn test_metrics_reset() {
    let metrics = AhfMetrics::new();

    metrics.record_hallucination();
    metrics.record_bias_block();
    metrics.record_request(50);

    let snapshot1 = metrics.snapshot();
    assert!(snapshot1.hallucination_count > 0);

    metrics.reset();
    let snapshot2 = metrics.snapshot();
    assert_eq!(snapshot2.hallucination_count, 0);
    assert_eq!(snapshot2.bias_blocked_count, 0);
    assert_eq!(snapshot2.total_requests, 0);
}

#[test]
fn test_metrics_per_model() {
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
fn test_metrics_clone() {
    let metrics = AhfMetrics::new();
    metrics.record_hallucination();

    let metrics2 = metrics.clone();
    let snapshot = metrics2.snapshot();
    assert_eq!(snapshot.hallucination_count, 1);
}

#[test]
fn test_audit_entry_creation() {
    let entry = AuditEntry::new(
        Uuid::new_v4(),
        "Accept".to_string(),
        0.85,
        true,
        0.90,
        0.1,
        "All checks passed".to_string(),
        "gpt-4".to_string(),
    );

    assert_eq!(entry.decision, "Accept");
    assert_eq!(entry.model_id, "gpt-4");
    assert!(!entry.hash.is_empty());
}

#[test]
fn test_audit_entry_integrity() {
    let entry = AuditEntry::new(
        Uuid::new_v4(),
        "Accept".to_string(),
        0.85,
        true,
        0.90,
        0.1,
        "All checks passed".to_string(),
        "gpt-4".to_string(),
    );

    assert!(entry.verify_integrity());
}

#[test]
fn test_audit_entry_with_user_and_session() {
    let user_id = "user123".to_string();
    let session_id = Uuid::new_v4();

    let entry = AuditEntry::new(
        Uuid::new_v4(),
        "Accept".to_string(),
        0.85,
        true,
        0.90,
        0.1,
        "Passed".to_string(),
        "gpt-4".to_string(),
    )
    .with_user_id(user_id.clone())
    .with_session_id(session_id);

    assert_eq!(entry.user_id, Some(user_id));
    assert_eq!(entry.session_id, Some(session_id));
}

#[test]
fn test_policy_default() {
    let policy = ahf_gateway::PolicyConfig::default();
    assert_eq!(policy.version, "1.0.0");
    assert_eq!(policy.grounding_accept_threshold, 0.8);
}

#[test]
fn test_policy_validation() {
    let mut policy = ahf_gateway::PolicyConfig::default();
    assert!(policy.validate().is_ok());

    policy.grounding_accept_threshold = 1.5;
    assert!(policy.validate().is_err());

    policy.grounding_accept_threshold = 0.8;
    policy.grounding_escalate_threshold = 0.9;
    assert!(policy.validate().is_err());
}

#[test]
fn test_policy_model_overrides() {
    let mut policy = ahf_gateway::PolicyConfig::default();
    let model_policy = ahf_gateway::ModelPolicy {
        model_id: "gpt-4".to_string(),
        grounding_threshold: Some(0.85),
        confidence_threshold: Some(0.7),
        bias_threshold: None,
        enable_shadow_mode: Some(true),
    };

    policy.add_model_override(model_policy);

    assert_eq!(policy.get_grounding_threshold("gpt-4"), 0.85);
    assert_eq!(policy.get_grounding_threshold("other"), 0.8);
    assert!(policy.is_shadow_mode_enabled("gpt-4"));
}

#[tokio::test]
async fn test_config_manager_update() {
    let cfg = AhfConfig::default();
    let manager = ConfigManager::new(cfg);

    let mut new_cfg = manager.get().await;
    new_cfg.grounding_threshold = 0.8;

    assert!(manager.update(new_cfg).await.is_ok());
    let updated = manager.get().await;
    assert_eq!(updated.grounding_threshold, 0.8);
}

#[tokio::test]
async fn test_config_manager_rollback() {
    let cfg = AhfConfig::default();
    let manager = ConfigManager::new(cfg.clone());

    let mut new_cfg = manager.get().await;
    new_cfg.grounding_threshold = 0.9;
    assert!(manager.update(new_cfg).await.is_ok());

    assert!(manager.rollback().await.is_ok());
    let rolled_back = manager.get().await;
    assert_eq!(rolled_back.grounding_threshold, 0.7);
}

#[tokio::test]
async fn test_config_manager_history() {
    let cfg = AhfConfig::default();
    let manager = ConfigManager::new(cfg);

    let mut cfg1 = manager.get().await;
    cfg1.grounding_threshold = 0.75;
    manager.update(cfg1).await.unwrap();

    let mut cfg2 = manager.get().await;
    cfg2.grounding_threshold = 0.8;
    manager.update(cfg2).await.unwrap();

    let history = manager.get_history().await;
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn test_audit_log_operations() {
    let log = AuditLog::new();
    let request_id = Uuid::new_v4();

    let entry = AuditEntry::new(
        request_id,
        "Accept".to_string(),
        0.85,
        true,
        0.90,
        0.1,
        "All checks passed".to_string(),
        "gpt-4".to_string(),
    );

    assert!(log.add(entry).await.is_ok());
    assert_eq!(log.count().await, 1);

    let entries = log.get_by_request_id(request_id).await;
    assert_eq!(entries.len(), 1);
}

#[tokio::test]
async fn test_audit_log_by_model() {
    let log = AuditLog::new();

    let entry1 = AuditEntry::new(
        Uuid::new_v4(),
        "Accept".to_string(),
        0.85,
        true,
        0.90,
        0.1,
        "Passed".to_string(),
        "gpt-4".to_string(),
    );

    let entry2 = AuditEntry::new(
        Uuid::new_v4(),
        "Accept".to_string(),
        0.80,
        true,
        0.85,
        0.15,
        "Passed".to_string(),
        "claude".to_string(),
    );

    log.add(entry1).await.unwrap();
    log.add(entry2).await.unwrap();

    let gpt4_entries = log.get_by_model_id("gpt-4").await;
    assert_eq!(gpt4_entries.len(), 1);

    let claude_entries = log.get_by_model_id("claude").await;
    assert_eq!(claude_entries.len(), 1);
}

#[tokio::test]
async fn test_audit_log_chain_integrity() {
    let log = AuditLog::new();

    let entry1 = AuditEntry::new(
        Uuid::new_v4(),
        "Accept".to_string(),
        0.85,
        true,
        0.90,
        0.1,
        "Passed".to_string(),
        "gpt-4".to_string(),
    );

    let entry2 = AuditEntry::new(
        Uuid::new_v4(),
        "Reject".to_string(),
        0.4,
        false,
        0.5,
        0.8,
        "Failed".to_string(),
        "gpt-3".to_string(),
    );

    log.add(entry1).await.unwrap();
    log.add(entry2).await.unwrap();

    assert!(log.verify_chain_integrity().await);
}
