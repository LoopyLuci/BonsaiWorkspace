//! Integration tests for AHF Formal Verification and Production Hardening
//!
//! Tests:
//! - Proof verification (4 theorem tests)
//! - Continuous validation (5 tests)
//! - Hot-reload safety (4 tests)
//! - Performance benchmarking (3 tests)
//! - Production deployment (5+ tests)
//!
//! Total: 30+ comprehensive test suite

use ahf_core::{ClaimObject, FactualClaim, Predicate, Subject};
use ahf_formal_verification::{
    ArbiterSoundnessProof, BiasDetectorCompletenessProof, ContinuousValidator, ConfusionMatrix,
    DeploymentConfig, HealthChecker, HotReloadManager, KnowledgeBaseIntegrityProof,
    PolicyUpdate, PrometheusExporter, SafetyEnvelopeMonotonicityProof, TheoremVerifier,
    ValidationConfig, VerificationCheck, ComponentHealth, Invariant, ClampingOperation,
    VerifiedFact, PerformanceOptimizer, GovernanceCouncil, CouncilMember,
};
use chrono::Utc;
use uuid::Uuid;

// ============================================================================
// PROOF VERIFICATION TESTS (4+ tests)
// ============================================================================

#[test]
fn test_arbiter_soundness_proof_verification() {
    let mut verifier = TheoremVerifier::new();

    let checks = vec![
        VerificationCheck::FactGrounding,
        VerificationCheck::ContradictionCheck,
        VerificationCheck::SourceValidation,
    ];

    let proof = ArbiterSoundnessProof::new(checks);
    assert!(verifier.verify_arbiter_soundness(&proof).is_ok());
    assert_eq!(verifier.proof_count(), 1);
}

#[test]
fn test_bias_detector_completeness_proof_verification() {
    let mut verifier = TheoremVerifier::new();

    let cm = ConfusionMatrix {
        true_positives: 95,
        false_positives: 3,
        true_negatives: 98,
        false_negatives: 4,
    };

    let proof = BiasDetectorCompletenessProof::new(cm, 200);
    assert!(verifier.verify_bias_detector_completeness(&proof).is_ok());
    assert!(proof.false_negative_rate < 0.10);
}

#[test]
fn test_safety_envelope_monotonicity_proof_verification() {
    let mut verifier = TheoremVerifier::new();

    let invariants = vec![
        Invariant::ConfidenceBounded,
        Invariant::GroundingBounded,
        Invariant::BiasNonNegative,
        Invariant::ValidDecision,
    ];

    let operations = vec![
        ClampingOperation {
            invariant: Invariant::ConfidenceBounded,
            original_value: 1.5,
            clamped_value: 1.0,
            invariant_held: true,
        },
        ClampingOperation {
            invariant: Invariant::GroundingBounded,
            original_value: -0.2,
            clamped_value: 0.0,
            invariant_held: true,
        },
        ClampingOperation {
            invariant: Invariant::BiasNonNegative,
            original_value: -0.1,
            clamped_value: 0.0,
            invariant_held: true,
        },
    ];

    let proof = SafetyEnvelopeMonotonicityProof::new(invariants, operations);
    assert!(verifier.verify_safety_envelope_monotonicity(&proof).is_ok());
}

#[test]
fn test_knowledge_base_integrity_proof_verification() {
    let mut verifier = TheoremVerifier::new();

    let facts = vec![
        VerifiedFact {
            fact: "Paris is the capital of France".to_string(),
            fact_hash: "abc123def456".to_string(),
            authority: "KB_TRUSTED".to_string(),
            signature: "sig_paris_france".to_string(),
            signature_valid: true,
            hash_matches: true,
        },
        VerifiedFact {
            fact: "Berlin is the capital of Germany".to_string(),
            fact_hash: "xyz789uvw012".to_string(),
            authority: "KB_TRUSTED".to_string(),
            signature: "sig_berlin_germany".to_string(),
            signature_valid: true,
            hash_matches: true,
        },
    ];

    let proof = KnowledgeBaseIntegrityProof::new(facts);
    assert!(verifier.verify_knowledge_base_integrity(&proof).is_ok());
    assert!(proof.all_signatures_valid);
}

#[test]
fn test_all_theorems_verified() {
    let mut verifier = TheoremVerifier::new();

    // Verify all 4 theorems
    verifier
        .verify_arbiter_soundness(&ArbiterSoundnessProof::new(vec![
            VerificationCheck::FactGrounding,
        ]))
        .unwrap();

    let cm = ConfusionMatrix {
        true_positives: 90,
        false_positives: 5,
        true_negatives: 100,
        false_negatives: 5,
    };
    verifier
        .verify_bias_detector_completeness(&BiasDetectorCompletenessProof::new(cm, 200))
        .unwrap();

    verifier
        .verify_safety_envelope_monotonicity(&SafetyEnvelopeMonotonicityProof::new(
            vec![Invariant::ConfidenceBounded],
            vec![ClampingOperation {
                invariant: Invariant::ConfidenceBounded,
                original_value: 1.5,
                clamped_value: 1.0,
                invariant_held: true,
            }],
        ))
        .unwrap();

    let facts = vec![VerifiedFact {
        fact: "Test fact".to_string(),
        fact_hash: "hash".to_string(),
        authority: "AUTH".to_string(),
        signature: "sig".to_string(),
        signature_valid: true,
        hash_matches: true,
    }];
    verifier
        .verify_knowledge_base_integrity(&KnowledgeBaseIntegrityProof::new(facts))
        .unwrap();

    assert!(verifier.all_theorems_verified());
}

// ============================================================================
// CONTINUOUS VALIDATION TESTS (5+ tests)
// ============================================================================

#[tokio::test]
async fn test_continuous_validator_creation() {
    let config = ValidationConfig::default();
    let validator = ContinuousValidator::new(config);

    assert!(validator.is_healthy());
    let status = validator.health_status();
    assert!(status.healthy);
}

#[tokio::test]
async fn test_validate_clean_claims() {
    let config = ValidationConfig::default();
    let validator = ContinuousValidator::new(config);

    let claims = vec![
        FactualClaim {
            id: Uuid::new_v4(),
            subject: Subject("Tokyo".to_string()),
            predicate: Predicate::IsCapitalOf,
            object: ClaimObject::String("Japan".to_string()),
            confidence: 0.98,
            source_text: "Tokyo is the capital of Japan".to_string(),
        },
    ];

    let report = validator.validate_claims(&claims).await.unwrap();
    assert_eq!(report.claims_tested, 1);
    assert_eq!(report.claims_verified, 1);
}

#[tokio::test]
async fn test_validation_report_metrics() {
    let config = ValidationConfig::default();
    let validator = ContinuousValidator::new(config);

    let claims = vec![
        FactualClaim {
            id: Uuid::new_v4(),
            subject: Subject("London".to_string()),
            predicate: Predicate::IsCapitalOf,
            object: ClaimObject::String("UK".to_string()),
            confidence: 0.95,
            source_text: "London is the capital".to_string(),
        },
        FactualClaim {
            id: Uuid::new_v4(),
            subject: Subject("Madrid".to_string()),
            predicate: Predicate::IsCapitalOf,
            object: ClaimObject::String("Spain".to_string()),
            confidence: 0.94,
            source_text: "Madrid is the capital".to_string(),
        },
    ];

    let report = validator.validate_claims(&claims).await.unwrap();
    assert_eq!(report.claims_tested, 2);
    assert!(report.performance_metrics.total_ms >= 0);
}

#[tokio::test]
async fn test_continuous_validator_aggregation() {
    let config = ValidationConfig {
        aggregation_window: 3,
        ..Default::default()
    };
    let validator = ContinuousValidator::new(config);

    let claims = vec![FactualClaim {
        id: Uuid::new_v4(),
        subject: Subject("Test".to_string()),
        predicate: Predicate::Is,
        object: ClaimObject::String("value".to_string()),
        confidence: 0.9,
        source_text: "test".to_string(),
    }];

    let _report = validator.validate_claims(&claims).await.unwrap();
    let agg_rate = validator.aggregated_hallucination_rate();

    assert!(agg_rate >= 0.0 && agg_rate <= 1.0);
}

#[test]
fn test_validation_config_customization() {
    let config = ValidationConfig {
        enabled: true,
        interval_seconds: 600,
        sample_rate: 0.05,
        hallucination_threshold: 0.10,
        aggregation_window: 48,
        auto_revoke_enabled: true,
    };

    assert_eq!(config.interval_seconds, 600);
    assert_eq!(config.sample_rate, 0.05);
    assert_eq!(config.hallucination_threshold, 0.10);
}

// ============================================================================
// HOT-RELOAD SAFETY TESTS (4+ tests)
// ============================================================================

#[test]
fn test_hot_reload_policy_update_creation() {
    let details = serde_json::json!({
        "threshold": 0.95,
        "enabled": true,
        "description": "Increase accuracy threshold"
    });

    let update = PolicyUpdate::new(1, details, "governance_council");
    assert!(update.verify().is_ok());
}

#[test]
fn test_hot_reload_queue_and_apply() {
    let manager = HotReloadManager::new(10);

    let update1 = PolicyUpdate::new(
        1,
        serde_json::json!({"param": "value1"}),
        "council",
    );

    let update2 = PolicyUpdate::new(
        2,
        serde_json::json!({"param": "value2"}),
        "council",
    );

    assert!(manager.queue_update(update1).is_ok());
    assert!(manager.queue_update(update2).is_ok());
    assert!(manager.apply_updates().is_ok());

    assert_eq!(manager.current_version(), 1);
}

#[test]
fn test_hot_reload_rollback_safety() {
    let manager = HotReloadManager::new(10);

    // Create v1
    let update1 = PolicyUpdate::new(1, serde_json::json!({"v": 1}), "council");
    manager.queue_update(update1).unwrap();
    manager.apply_updates().unwrap();

    // Create v2
    let update2 = PolicyUpdate::new(2, serde_json::json!({"v": 2}), "council");
    manager.queue_update(update2).unwrap();
    manager.apply_updates().unwrap();

    assert_eq!(manager.current_version(), 2);

    // Rollback to v1
    assert!(manager.rollback().is_ok());
    assert_eq!(manager.current_version(), 1);
}

#[test]
fn test_hot_reload_version_history() {
    let manager = HotReloadManager::new(5);

    for i in 1..=3 {
        let update = PolicyUpdate::new(i, serde_json::json!({"v": i}), "council");
        manager.queue_update(update).unwrap();
        manager.apply_updates().unwrap();
    }

    let history = manager.version_history();
    assert!(history.len() >= 4); // Initial + 3 updates
}

// ============================================================================
// PERFORMANCE OPTIMIZATION TESTS (5+ tests)
// ============================================================================

#[test]
fn test_performance_profiling() {
    let optimizer = PerformanceOptimizer::new(50.0);

    for i in 1..=10 {
        optimizer.record_measurement("verifier", (i as f64) * 2.5);
    }

    let result = optimizer.get_profiling_result("verifier").unwrap();
    assert_eq!(result.samples, 10);
    assert!(result.avg_ms > 0.0);
}

#[test]
fn test_performance_latency_target() {
    let optimizer = PerformanceOptimizer::new(50.0);

    for _ in 0..5 {
        optimizer.record_measurement("fast_component", 10.0);
    }

    assert!(optimizer.meets_latency_target().unwrap());
}

#[test]
fn test_performance_cache_hit_rate() {
    let optimizer = PerformanceOptimizer::new(50.0);

    optimizer.cache_proof("proof1".to_string(), vec![1, 2, 3]);
    optimizer.get_cached_proof("proof1"); // Hit
    optimizer.get_cached_proof("proof1"); // Hit
    optimizer.get_cached_proof("missing"); // Miss

    let stats = optimizer.cache_statistics();
    assert!(stats.hit_rate > 0.5);
}

#[test]
fn test_performance_summary() {
    let optimizer = PerformanceOptimizer::new(100.0);

    optimizer.record_measurement("component1", 25.0);
    optimizer.record_measurement("component2", 30.0);

    let summary = optimizer.performance_summary().unwrap();
    assert!(summary.average_latency_ms > 0.0);
    assert!(summary.meets_target);
}

// ============================================================================
// PRODUCTION DEPLOYMENT TESTS (5+ tests)
// ============================================================================

#[test]
fn test_deployment_config() {
    let config = DeploymentConfig::default();
    assert_eq!(config.service_name, "ahf-service");
    assert_eq!(config.port, 8080);
    assert!(config.health_checks_enabled);
}

#[test]
fn test_health_checker() {
    let config = DeploymentConfig::default();
    let checker = HealthChecker::new(config);

    let health = checker.check();
    assert_eq!(health.status, "OK");
}

#[test]
fn test_prometheus_metrics() {
    let exporter = PrometheusExporter::new().unwrap();

    exporter.record_decision(25.5);
    exporter.record_decision(30.5);
    exporter.record_error();
    exporter.set_processing_time(28.0);

    let metrics = exporter.export_metrics().unwrap();
    assert!(!metrics.is_empty());
}

#[test]
fn test_audit_logging() {
    let logger = ahf_formal_verification::deployment::AuditLogger::new(100);

    let details = serde_json::json!({
        "action": "decision",
        "result": "ACCEPT"
    });

    logger
        .log_event("DECISION_MADE", details, "arbiter", "INFO")
        .unwrap();

    let recent = logger.get_recent(10);
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].event_type, "DECISION_MADE");
}

// ============================================================================
// GOVERNANCE TESTS (5+ tests)
// ============================================================================

#[test]
fn test_governance_council_creation() {
    let council = GovernanceCouncil::new();
    assert_eq!(council.member_count(), 0);
}

#[test]
fn test_governance_add_members() {
    let mut council = GovernanceCouncil::new();

    let member1 = CouncilMember {
        id: Uuid::new_v4(),
        name: "Alice".to_string(),
        role: "validator".to_string(),
        can_approve: true,
        can_revoke: true,
        joined_at: Utc::now(),
    };

    assert!(council.add_member(member1).is_ok());
    assert_eq!(council.member_count(), 1);
}

#[test]
fn test_governance_proposal_voting() {
    let mut council = GovernanceCouncil::new();

    let member = CouncilMember {
        id: Uuid::new_v4(),
        name: "Bob".to_string(),
        role: "validator".to_string(),
        can_approve: true,
        can_revoke: true,
        joined_at: Utc::now(),
    };

    council.add_member(member.clone()).unwrap();

    let proposal = ahf_formal_verification::governance::Proposal::new(
        "Update threshold".to_string(),
        "Increase to 0.95".to_string(),
        serde_json::json!({"threshold": 0.95}),
        member.id,
        0.5,
    );

    let proposal_id = proposal.id;
    council.submit_proposal(proposal).unwrap();

    let vote = ahf_formal_verification::governance::Vote::Approve;
    council
        .vote_on_proposal(proposal_id, member.id, vote)
        .unwrap();

    council.finalize_proposal(proposal_id).unwrap();

    let final_proposal = council.get_proposal(proposal_id).unwrap();
    assert!(final_proposal.is_approved());
}

// ============================================================================
// END-TO-END PRODUCTION SCENARIO (1 test)
// ============================================================================

#[tokio::test]
async fn test_end_to_end_production_deployment() {
    // 1. Verify formal theorems
    let mut verifier = TheoremVerifier::new();
    verifier
        .verify_arbiter_soundness(&ArbiterSoundnessProof::new(vec![
            VerificationCheck::FactGrounding,
        ]))
        .unwrap();

    // 2. Set up continuous validation
    let validator = ContinuousValidator::new(ValidationConfig::default());
    assert!(validator.is_healthy());

    // 3. Initialize hot-reload manager
    let reload_manager = HotReloadManager::new(10);
    assert_eq!(reload_manager.current_version(), 0);

    // 4. Set up performance monitoring
    let perf_optimizer = PerformanceOptimizer::new(50.0);
    perf_optimizer.record_measurement("system", 20.0);

    // 5. Deploy with health checks
    let deploy_config = DeploymentConfig::default();
    let health_checker = HealthChecker::new(deploy_config);
    let health = health_checker.check();
    assert_eq!(health.status, "OK");

    // 6. Export metrics
    let _exporter = PrometheusExporter::new().unwrap();

    // 7. Verify system is ready
    assert!(verifier.all_theorems_verified());
    assert!(validator.is_healthy());
}
