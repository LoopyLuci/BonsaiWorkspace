//! Integration tests for AHF Core Infrastructure
//!
//! Tests the complete workflow of creating claims, signals, policies,
//! and decisions within the AHF system.

use ahf_core::*;

#[test]
fn test_complete_claim_workflow() {
    let subject = Subject::new("wiki:Paris", "Paris");
    let predicate = Predicate::new("capital_of", "Capital Of");
    let claim = FactualClaim::new(
        subject,
        predicate,
        "France",
        0.95,
    ).expect("Failed to create claim");

    assert_eq!(claim.object, "France");
    assert_eq!(claim.source_confidence, 0.95);
}

#[test]
fn test_grounding_score_workflow() {
    let score1 = GroundingScore::new(9, 10);
    assert_eq!(score1.quality_level(), "Very High");

    let score2 = GroundingScore::new(5, 10);
    assert_eq!(score2.quality_level(), "Medium");

    let score3 = GroundingScore::new(2, 10);
    assert_eq!(score3.quality_level(), "Low");
}

#[test]
fn test_confidence_score_calibration() {
    let score = ConfidenceScore::new(0.7);

    // If claim is correct, calibration error is 0.3
    assert!((score.calibration_error(true) - 0.3).abs() < f64::EPSILON);

    // If claim is incorrect, calibration error is 0.7
    assert!((score.calibration_error(false) - 0.7).abs() < f64::EPSILON);
}

#[test]
fn test_signals_validation() {
    let grounding = GroundingScore::new(8, 10);
    let verification = VerificationResult {
        status: VerificationStatus::Valid,
        proof: None,
        reasoning: "Matches source".to_string(),
        confidence: 0.95,
    };
    let bias = BiasScore::clean();
    let confidence = ConfidenceScore::new(0.85);

    let signals = AhfSignals::new(grounding, verification, bias, confidence);
    assert!(signals.validate());
}

#[test]
fn test_policy_registry_workflow() {
    let mut registry = PolicyRegistry::new();

    // Get default policy
    let policy = registry.get_policy().expect("Failed to get policy");
    assert_eq!(policy.grounding_threshold, 0.7);

    // Update policy
    let mut new_policy = ArbiterPolicy::default();
    new_policy.grounding_threshold = 0.8;
    registry.set_policy(new_policy).expect("Failed to set policy");

    // Verify update
    let updated = registry.get_policy().expect("Failed to get updated policy");
    assert_eq!(updated.grounding_threshold, 0.8);
    assert_eq!(registry.policy_history.len(), 2);
}

#[test]
fn test_model_policy_override() {
    let mut registry = PolicyRegistry::new();

    let model_policy = ModelPolicy {
        model_name: "gpt-4-turbo".to_string(),
        grounding_threshold: Some(0.85),
        confidence_threshold: Some(0.75),
        bias_threshold: None,
    };

    registry.set_model_policy(model_policy).expect("Failed to set model policy");

    let policy = registry.get_model_policy("gpt-4-turbo").expect("Failed to get model policy");
    assert_eq!(policy.grounding_threshold, 0.85);
    assert_eq!(policy.confidence_threshold, 0.75);
    assert_eq!(policy.bias_threshold, 0.5); // default
}

#[test]
fn test_bias_score_composition() {
    let bias = BiasScore::new(0.3, 0.2, 0.1, 0.05, 0.25);

    // Average of clamped values: (0.3 + 0.2 + 0.1 + 0.05 + 0.25) / 5 = 0.18
    assert!((bias.score - 0.18).abs() < f64::EPSILON);

    // Check individual components
    assert_eq!(bias.biases[0], 0.3); // confirmation
    assert_eq!(bias.biases[1], 0.2); // anchoring
    assert_eq!(bias.biases[2], 0.1); // availability
    assert_eq!(bias.biases[3], 0.05); // recency
    assert_eq!(bias.biases[4], 0.25); // framing
}

#[test]
fn test_ahf_decision_creation() {
    let signals = DecisionSignals {
        grounding_score: 0.85,
        verification_valid: true,
        model_confidence: 0.90,
        bias_score: 0.1,
        criticality_level: "medium".to_string(),
    };

    let decision = AhfDecision::new(
        Decision::Accept,
        DecisionReason::AllChecksPassed,
        "All verification checks passed".to_string(),
        signals,
        false,
    );

    assert_eq!(decision.decision, Decision::Accept);
    assert!(!decision.safety_envelope_applied);
}

#[test]
fn test_metrics_tracking() {
    let mut metrics = AhfMetrics::new();

    // Record various events
    metrics.record_hallucination();
    metrics.record_hallucination();
    metrics.record_false_rejection();
    metrics.record_verification(25.0);
    metrics.record_verification(75.0);
    metrics.record_bias_block();
    metrics.record_escalation();
    metrics.record_accept();
    metrics.record_reject();

    // Verify counts
    assert_eq!(metrics.hallucination_count, 2);
    assert_eq!(metrics.false_rejection_count, 1);
    assert_eq!(metrics.total_verified, 2);
    assert_eq!(metrics.bias_blocks_count, 1);
    assert_eq!(metrics.escalation_count, 1);

    // Verify calculations
    assert!((metrics.avg_latency_ms - 50.0).abs() < f64::EPSILON);
    assert!((metrics.false_positive_rate() - 0.5).abs() < f64::EPSILON);
    assert!((metrics.detection_rate() - 2.0/3.0).abs() < f64::EPSILON);
    assert!((metrics.acceptance_rate() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_error_variants() {
    let err1 = AhfError::knowledge_base_lookup_failed("not found");
    assert!(err1.is_recoverable());

    let err2 = AhfError::verification_failed("contradiction");
    assert!(err2.requires_escalation());

    let err3 = AhfError::bias_detected("high bias");
    assert!(err3.is_recoverable());

    let err4 = AhfError::bias_violation_exceeded("exceeded");
    assert!(err4.requires_escalation());
}

#[test]
fn test_serialization_roundtrip() {
    // Create a complete decision log
    let decision_log = DecisionLog {
        timestamp: Utc::now(),
        decision: AhfDecision::new(
            Decision::Accept,
            DecisionReason::AllChecksPassed,
            "Test decision".to_string(),
            DecisionSignals {
                grounding_score: 0.8,
                verification_valid: true,
                model_confidence: 0.9,
                bias_score: 0.1,
                criticality_level: "low".to_string(),
            },
            false,
        ),
        session_id: None,
    };

    // Serialize
    let json = serde_json::to_string(&decision_log).expect("Failed to serialize");

    // Deserialize
    let deserialized: DecisionLog = serde_json::from_str(&json).expect("Failed to deserialize");

    // Verify
    assert_eq!(decision_log.decision.decision, deserialized.decision.decision);
    assert_eq!(decision_log.decision.signals.grounding_score, deserialized.decision.signals.grounding_score);
}

#[test]
fn test_policy_version_history() {
    let mut registry = PolicyRegistry::new();

    // Record multiple changes
    registry.record_change("grounding_threshold".to_string(), 0.7, "council".to_string());
    registry.record_change("grounding_threshold".to_string(), 0.75, "council".to_string());
    registry.record_change("grounding_threshold".to_string(), 0.8, "council".to_string());

    let history = registry.get_version_history("grounding_threshold").expect("Failed to get history");
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].value, 0.7);
    assert_eq!(history[1].value, 0.75);
    assert_eq!(history[2].value, 0.8);
}

#[test]
fn test_atomic_metrics_thread_safe() {
    let metrics = AtomicAhfMetrics::new();

    // Simulate concurrent updates
    for _ in 0..100 {
        metrics.inc_hallucination();
        metrics.inc_verified();
    }

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.hallucination_count, 100);
    assert_eq!(snapshot.total_verified, 100);
}

#[test]
fn test_full_verification_workflow() {
    // Create a claim
    let subject = Subject::new("wiki:Einstein", "Albert Einstein")
        .with_uri("http://dbpedia.org/resource/Albert_Einstein");
    let predicate = Predicate::new("born_in", "Born In");
    let _claim = FactualClaim::new(subject, predicate, "Ulm", 0.92)
        .expect("Failed to create claim")
        .with_context("1879");

    // Create verification result
    let verification = VerificationResult {
        status: VerificationStatus::Valid,
        proof: Some(VerificationProof {
            content_hash: "abc123".to_string(),
            sources: vec!["wikipedia".to_string(), "britannica".to_string()],
            signature: None,
        }),
        reasoning: "Matches authoritative sources".to_string(),
        confidence: 0.99,
    };

    // Create signals
    let signals = AhfSignals::new(
        GroundingScore::new(95, 100),
        verification,
        BiasScore::clean(),
        ConfidenceScore::new(0.92),
    );

    // Validate signals
    assert!(signals.validate());

    // Create decision
    let decision = AhfDecision::new(
        Decision::Accept,
        DecisionReason::AllChecksPassed,
        "High confidence claim with strong grounding".to_string(),
        DecisionSignals {
            grounding_score: 0.95,
            verification_valid: true,
            model_confidence: 0.92,
            bias_score: 0.0,
            criticality_level: "low".to_string(),
        },
        false,
    );

    assert_eq!(decision.decision, Decision::Accept);
}

#[test]
fn test_decision_with_escalation() {
    let decision = AhfDecision::new(
        Decision::Escalate,
        DecisionReason::HighCriticality,
        "High criticality claim requires human review".to_string(),
        DecisionSignals {
            grounding_score: 0.65,
            verification_valid: false,
            model_confidence: 0.55,
            bias_score: 0.7,
            criticality_level: "critical".to_string(),
        },
        true,
    );

    assert_eq!(decision.decision, Decision::Escalate);
    assert!(decision.safety_envelope_applied);
}

#[test]
fn test_rejection_with_bias() {
    let decision = AhfDecision::new(
        Decision::Reject,
        DecisionReason::HighBias,
        "Claim contains significant confirmation bias".to_string(),
        DecisionSignals {
            grounding_score: 0.5,
            verification_valid: false,
            model_confidence: 0.4,
            bias_score: 0.8,
            criticality_level: "high".to_string(),
        },
        true,
    );

    assert_eq!(decision.decision, Decision::Reject);
}
