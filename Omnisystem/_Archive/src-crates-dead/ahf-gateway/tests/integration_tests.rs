//! Integration tests for AHG

use ahf_gateway::{
    AhgActor, AhfConfig, ConfigManager, AuditLog, AhfMetrics, Criticality, spawn_actor,
    messages::VerifyOutput,
};

#[tokio::test]
async fn test_full_gateway_workflow() {
    let config = AhfConfig::default();
    let actor = AhgActor::new(config).await.expect("failed to create actor");
    let handle = spawn_actor(actor);

    let msg = VerifyOutput {
        output: "Paris is the capital of France.".to_string(),
        model_id: "gpt-4".to_string(),
        criticality: Criticality::High,
    };

    let result = handle.actor_ref().send(msg);
    assert!(result.is_ok());

    // Give the actor time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_config_manager_integration() {
    let config = AhfConfig::default();
    let manager = ConfigManager::new(config);

    let current = manager.get().await;
    assert_eq!(current.grounding_threshold, 0.7);

    let mut new_config = current.clone();
    new_config.grounding_threshold = 0.8;

    let result = manager.update(new_config).await;
    assert!(result.is_ok());

    let updated = manager.get().await;
    assert_eq!(updated.grounding_threshold, 0.8);
}

#[tokio::test]
async fn test_audit_log_integration() {
    let log = AuditLog::new();
    let request_id = uuid::Uuid::new_v4();

    let entry = ahf_gateway::AuditEntry::new(
        request_id,
        "Accept".to_string(),
        0.85,
        true,
        0.90,
        0.1,
        "All checks passed".to_string(),
        "gpt-4".to_string(),
    );

    let result = log.add(entry.clone()).await;
    assert!(result.is_ok());

    let entries = log.get_by_request_id(request_id).await;
    assert_eq!(entries.len(), 1);

    let all = log.get_all().await;
    assert_eq!(all.len(), 1);
}

#[tokio::test]
async fn test_metrics_integration() {
    let metrics = AhfMetrics::new();

    metrics.record_request(25);
    metrics.record_hallucination();
    metrics.record_model_metrics("gpt-4", 0.95, 0.85, 0.1, false);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.total_requests, 1);
    assert_eq!(snapshot.hallucination_count, 1);
    assert!(snapshot.per_model_metrics.contains_key("gpt-4"));
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = AhfConfig::default();
    assert!(config.validate().is_ok());

    config.grounding_threshold = 1.5;
    assert!(config.validate().is_err());

    config.grounding_threshold = 0.7;
    config.confidence_threshold = -0.5;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_policy_per_model_overrides() {
    let config = AhfConfig::default();
    let mut policy = config.policy.clone();

    let model_policy = ahf_gateway::ModelPolicy {
        model_id: "gpt-4".to_string(),
        grounding_threshold: Some(0.85),
        confidence_threshold: Some(0.7),
        bias_threshold: None,
        enable_shadow_mode: Some(true),
    };

    policy.add_model_override(model_policy);

    assert_eq!(policy.get_grounding_threshold("gpt-4"), 0.85);
    assert_eq!(policy.get_grounding_threshold("other"), config.policy.grounding_accept_threshold);
}

#[tokio::test]
async fn test_config_serialization_roundtrip() {
    let config = AhfConfig::default();
    let json = serde_json::to_string(&config).expect("serialization failed");
    let config2: AhfConfig = serde_json::from_str(&json).expect("deserialization failed");

    assert_eq!(config.grounding_threshold, config2.grounding_threshold);
    assert_eq!(config.pipeline_timeout_ms, config2.pipeline_timeout_ms);
}

#[tokio::test]
async fn test_audit_chain_integrity() {
    let log = AuditLog::new();

    for i in 0..5 {
        let entry = ahf_gateway::AuditEntry::new(
            uuid::Uuid::new_v4(),
            format!("Decision{}", i),
            0.75,
            true,
            0.80,
            0.15,
            format!("Entry {}", i),
            "gpt-4".to_string(),
        );
        let _ = log.add(entry).await;
    }

    let is_valid = log.verify_chain_integrity().await;
    assert!(is_valid);
}

#[tokio::test]
async fn test_metrics_per_model_tracking() {
    let metrics = AhfMetrics::new();

    metrics.record_model_metrics("gpt-4", 0.95, 0.85, 0.1, false);
    metrics.record_model_metrics("gpt-4", 0.90, 0.80, 0.2, true);
    metrics.record_model_metrics("claude", 0.92, 0.88, 0.12, false);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.per_model_metrics.len(), 2);

    let gpt4_metrics = snapshot.per_model_metrics.get("gpt-4").unwrap();
    assert_eq!(gpt4_metrics.requests, 2);
    assert_eq!(gpt4_metrics.hallucinations, 1);

    let claude_metrics = snapshot.per_model_metrics.get("claude").unwrap();
    assert_eq!(claude_metrics.requests, 1);
}

#[tokio::test]
async fn test_config_rollback() {
    let config = AhfConfig::default();
    let manager = ConfigManager::new(config);

    let mut config1 = manager.get().await;
    config1.grounding_threshold = 0.75;
    manager.update(config1).await.unwrap();

    let mut config2 = manager.get().await;
    config2.grounding_threshold = 0.85;
    manager.update(config2).await.unwrap();

    let current = manager.get().await;
    assert_eq!(current.grounding_threshold, 0.85);

    manager.rollback().await.unwrap();
    let rolled_back = manager.get().await;
    assert_eq!(rolled_back.grounding_threshold, 0.75);
}

#[tokio::test]
async fn test_false_rejection_rate_calculation() {
    let metrics = AhfMetrics::new();

    metrics.record_request(10);
    metrics.record_request(10);
    metrics.record_request(10);
    metrics.record_false_rejection();

    let rate = metrics.false_rejection_rate();
    assert!((rate - 1.0/3.0).abs() < 0.01);
}

#[tokio::test]
async fn test_audit_log_by_user_id() {
    let log = AuditLog::new();

    let entry1 = ahf_gateway::AuditEntry::new(
        uuid::Uuid::new_v4(),
        "Accept".to_string(),
        0.85,
        true,
        0.90,
        0.1,
        "Passed".to_string(),
        "gpt-4".to_string(),
    ).with_user_id("user123".to_string());

    let entry2 = ahf_gateway::AuditEntry::new(
        uuid::Uuid::new_v4(),
        "Accept".to_string(),
        0.80,
        true,
        0.85,
        0.15,
        "Passed".to_string(),
        "gpt-4".to_string(),
    ).with_user_id("user456".to_string());

    log.add(entry1).await.unwrap();
    log.add(entry2).await.unwrap();

    let user123_entries = log.get_by_user_id("user123").await;
    assert_eq!(user123_entries.len(), 1);

    let user456_entries = log.get_by_user_id("user456").await;
    assert_eq!(user456_entries.len(), 1);
}
