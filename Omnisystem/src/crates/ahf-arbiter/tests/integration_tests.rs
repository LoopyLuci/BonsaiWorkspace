//! Integration tests for the AHF Arbiter
//!
//! Tests the full end-to-end flow including decision making and safety envelopes.

use ahf_arbiter::{Arbiter, Decision};
use ahf_core::{
    AhfSignals, BiasScore, ConfidenceScore, Criticality, GroundingScore, VerificationResult,
    VerificationStatus, ArbiterPolicy, ModelPolicy,
};

fn make_signals(
    grounding: f64,
    confidence: f64,
    bias: f64,
) -> AhfSignals {
    AhfSignals {
        grounding_score: GroundingScore::new(
            (grounding * 10.0) as usize,
            10,
        ),
        verification_result: VerificationResult {
            status: VerificationStatus::Valid,
            proof: None,
            reasoning: "All claims verified against sources".to_string(),
            confidence: 0.95,
        },
        model_confidence: ConfidenceScore::new(confidence),
        bias_score: BiasScore::new(bias, bias, bias, bias, bias),
    }
}

#[tokio::test]
async fn test_full_arbitration_flow_accept() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.90, 0.1);
    let output = "Paris is the capital of France.";

    let (decision, safe_output) = arbiter
        .arbitrate(&signals, Some(output), Criticality::Low)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Accept);
    assert!(safe_output.is_some());
    assert_eq!(safe_output.unwrap(), output);
    assert!(decision.safety_envelope_applied);
}

#[tokio::test]
async fn test_full_arbitration_flow_reject_low_grounding() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.4, 0.90, 0.1);

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Low)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Reject);
}

#[tokio::test]
async fn test_full_arbitration_flow_reject_harmful_output() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.90, 0.1);
    let output = "I am certain that this is true.";

    let (decision, safe_output) = arbiter
        .arbitrate(&signals, Some(output), Criticality::Low)
        .await
        .unwrap();

    // The certainty expression should be removed/replaced
    assert_eq!(decision.decision, Decision::Accept);
    assert!(!safe_output.unwrap().contains("I am certain that"));
}

#[tokio::test]
async fn test_full_arbitration_flow_escalate() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.75, 0.90, 0.1);

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::High)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Escalate);
}

#[tokio::test]
async fn test_policy_update_affects_decisions() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.75, 0.90, 0.1); // 0.75 grounding

    // With default policy (0.7 threshold), this should accept
    let (decision1, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Low)
        .await
        .unwrap();
    assert_eq!(decision1.decision, Decision::Accept);

    // Update policy to require 0.8 grounding
    let mut new_policy = ArbiterPolicy::default();
    new_policy.grounding_threshold = 0.8;
    arbiter.update_policy(new_policy, "council").unwrap();

    // Now same signals should reject
    let (decision2, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Low)
        .await
        .unwrap();
    assert_eq!(decision2.decision, Decision::Reject);
}

#[tokio::test]
async fn test_model_policy_override() {
    let mut arbiter = Arbiter::new().unwrap();

    let model_policy = ModelPolicy {
        model_name: "strict-model".to_string(),
        grounding_threshold: Some(0.95),
        confidence_threshold: Some(0.95),
        bias_threshold: None,
    };

    arbiter.set_model_policy(model_policy).unwrap();

    let policy = arbiter.get_model_policy("strict-model").unwrap();
    assert_eq!(policy.grounding_threshold, 0.95);
    assert_eq!(policy.confidence_threshold, 0.95);
}

#[tokio::test]
async fn test_safety_envelope_certainty_replacement() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.90, 0.1);
    let output = "I am absolutely confident that Paris is in France.";

    let (decision, safe_output) = arbiter
        .arbitrate(&signals, Some(output), Criticality::Low)
        .await
        .unwrap();

    let safe = safe_output.unwrap();
    assert!(!safe.contains("absolutely confident"));
    assert!(safe.contains("Based on verified sources"));
}

#[tokio::test]
async fn test_multiple_safety_violations_replaced() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.90, 0.1);
    let output = "I am certain that X and I am guaranteed that Y.";

    let (decision, safe_output) = arbiter
        .arbitrate(&signals, Some(output), Criticality::Low)
        .await
        .unwrap();

    // Both certainty expressions should be replaced/removed
    assert_eq!(decision.decision, Decision::Accept);
    let safe = safe_output.unwrap();
    assert!(!safe.contains("I am certain that"));
    assert!(!safe.contains("I am guaranteed that"));
}

#[tokio::test]
async fn test_decision_with_medium_criticality() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.8, 0.85, 0.2);

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Medium)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Accept);
}

#[tokio::test]
async fn test_decision_with_critical_criticality() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.95, 0.95, 0.1);

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Critical)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Accept); // Perfect grounding allows accept
}

#[tokio::test]
async fn test_audit_trail_in_decision() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.90, 0.15);

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Low)
        .await
        .unwrap();

    // Check that decision contains all signals (0.85 * 10 = 8, so expect 0.8)
    assert!((decision.signals.grounding_score - 0.8).abs() < f64::EPSILON);
    assert!((decision.signals.model_confidence - 0.90).abs() < f64::EPSILON);
    assert!(decision.signals.verification_valid);
}

#[tokio::test]
async fn test_session_id_unique() {
    let arbiter1 = Arbiter::new().unwrap();
    let arbiter2 = Arbiter::new().unwrap();

    assert_ne!(arbiter1.session_id(), arbiter2.session_id());
}

#[tokio::test]
async fn test_policy_history_tracking() {
    let mut arbiter = Arbiter::new().unwrap();

    let mut policy1 = ArbiterPolicy::default();
    policy1.grounding_threshold = 0.7;
    arbiter.update_policy(policy1, "council_v1").unwrap();

    let mut policy2 = ArbiterPolicy::default();
    policy2.grounding_threshold = 0.8;
    arbiter.update_policy(policy2, "council_v2").unwrap();

    let history = arbiter
        .policy_history("grounding_threshold")
        .unwrap();

    assert!(history.len() >= 1);
}

#[tokio::test]
async fn test_low_confidence_rejection() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.4, 0.1); // Low confidence

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Low)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Reject);
}

#[tokio::test]
async fn test_high_bias_rejection() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.90, 0.9); // High bias

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Low)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Reject);
}

#[tokio::test]
async fn test_perfect_signals_acceptance() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(1.0, 1.0, 0.0); // Perfect signals

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Critical)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Accept);
}

#[tokio::test]
async fn test_contradicted_verification_rejection() {
    let mut arbiter = Arbiter::new().unwrap();
    let mut signals = make_signals(0.85, 0.90, 0.1);
    signals.verification_result.status = VerificationStatus::Invalid;

    let (decision, _) = arbiter
        .arbitrate(&signals, Some("output"), Criticality::Low)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Reject);
}

#[tokio::test]
async fn test_long_output_truncation() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.90, 0.1);
    let long_output = "a".repeat(20000); // Very long output

    let (decision, safe_output) = arbiter
        .arbitrate(&signals, Some(&long_output), Criticality::Low)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Accept);
    let safe = safe_output.unwrap_or_else(|| panic!("Expected output but got None"));
    assert!(safe.len() <= 10000); // Should be truncated to max
}

#[tokio::test]
async fn test_no_output_text_no_safety_envelope() {
    let mut arbiter = Arbiter::new().unwrap();
    let signals = make_signals(0.85, 0.90, 0.1);

    let (decision, safe_output) = arbiter
        .arbitrate(&signals, None, Criticality::Low)
        .await
        .unwrap();

    assert_eq!(decision.decision, Decision::Accept);
    assert!(safe_output.is_none());
    assert!(!decision.safety_envelope_applied);
}
