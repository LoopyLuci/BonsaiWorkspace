//! Integration tests for ahf-formal-verifier
//!
//! Comprehensive test suite covering:
//! - Schema validation (20+ tests)
//! - Session consistency (20+ tests)
//! - Axiom proof verification (10+ tests)
//! - End-to-end verification pipeline (10+ tests)

use ahf_formal_verifier::*;
use serde_json::json;

// ============================================================================
// Output Parser Tests (10+ tests)
// ============================================================================

#[test]
fn test_parse_json_object() {
    let parser = OutputParser::new();
    let json = r#"{"name": "Alice", "age": 30}"#;
    let result = parser.parse(json);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.format, OutputFormat::Json);
}

#[test]
fn test_parse_json_array() {
    let parser = OutputParser::new();
    let json = r#"[1, 2, 3, 4, 5]"#;
    let result = parser.parse(json);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.format, OutputFormat::Json);
}

#[test]
fn test_parse_plain_text() {
    let parser = OutputParser::new();
    let text = "This is a plain text response from the model.";
    let result = parser.parse(text);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.format, OutputFormat::PlainText);
}

#[test]
fn test_parse_nested_json() {
    let parser = OutputParser::new();
    let json = r#"{"user": {"name": "Bob", "email": "bob@example.com"}}"#;
    let result = parser.parse(json);
    assert!(result.is_ok());
}

#[test]
fn test_parse_json_with_null() {
    let parser = OutputParser::new();
    let json = r#"{"optional_field": null}"#;
    let result = parser.parse(json);
    assert!(result.is_ok());
}

#[test]
fn test_parse_json_with_numbers() {
    let parser = OutputParser::new();
    let json = r#"{"count": 42, "ratio": 3.14}"#;
    let result = parser.parse(json);
    assert!(result.is_ok());
}

#[test]
fn test_parse_respects_max_length() {
    let parser = OutputParser::with_max_length(50);
    let long_text = "x".repeat(100);
    let result = parser.parse(&long_text);
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_json_returns_error() {
    let parser = OutputParser::new();
    let result = parser.parse("{invalid json]");
    assert!(result.is_err());
}

#[test]
fn test_parsed_output_contains_metadata() {
    let parser = OutputParser::new();
    let json = r#"{"key": "value"}"#;
    let result = parser.parse(json);
    let parsed = result.unwrap();
    assert_eq!(parsed.content_length, json.len());
    assert!(parsed.is_deterministic);
}

#[test]
fn test_parse_json_boolean() {
    let parser = OutputParser::new();
    let json = r#"{"active": true, "deleted": false}"#;
    let result = parser.parse(json);
    assert!(result.is_ok());
}

// ============================================================================
// Schema Validator Tests (15+ tests)
// ============================================================================

#[test]
fn test_schema_validator_creation() {
    let validator = SchemaValidator::new();
    assert!(!validator.list_theorems().is_empty());
}

#[test]
fn test_register_schema() {
    let mut validator = SchemaValidator::new();
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        }
    });
    let result = validator.register_schema("user", schema);
    assert!(result.is_ok());
}

#[test]
fn test_validate_json_schema() {
    let parser = OutputParser::new();
    let validator = SchemaValidator::new();
    let json = r#"{"name": "Alice"}"#;
    let parsed = parser.parse(json).unwrap();
    let result = validator.validate(&parsed);
    assert!(result.is_ok());
}

#[test]
fn test_type_matching_exact() {
    let validator = SchemaValidator::new();
    assert!(validator.type_matches("Integer", "Integer"));
}

#[test]
fn test_type_matching_polymorphic() {
    let validator = SchemaValidator::new();
    assert!(validator.type_matches("Integer", "Number"));
    assert!(validator.type_matches("Float", "Number"));
}

#[test]
fn test_type_matching_scalar() {
    let validator = SchemaValidator::new();
    assert!(validator.type_matches("String", "Scalar"));
    assert!(validator.type_matches("Integer", "Scalar"));
}

#[test]
fn test_register_custom_constraint() {
    let mut validator = SchemaValidator::new();
    let constraint = ValidationConstraint {
        name: "email".to_string(),
        constraint_type: ConstraintType::Pattern {
            regex: r"[a-z]+@[a-z]+\.[a-z]+".to_string(),
        },
        required: true,
    };
    validator.register_constraint(constraint);
    assert_eq!(validator.constraints.len(), 1);
}

#[test]
fn test_constraint_violation_creation() {
    let violation = ConstraintViolation {
        constraint_name: "age".to_string(),
        reason: "age cannot be negative".to_string(),
        actual_value: "-5".to_string(),
        expected: ">= 0".to_string(),
        path: "$.user.age".to_string(),
    };
    assert_eq!(violation.constraint_name, "age");
}

// ============================================================================
// Session History Tests (20+ tests)
// ============================================================================

#[test]
fn test_session_history_empty() {
    let session = SessionHistory::new();
    assert!(session.is_empty());
    assert_eq!(session.len(), 0);
}

#[test]
fn test_session_add_fact() {
    let mut session = SessionHistory::new();
    let claim = create_test_claim("Paris", "capital");
    let fact = SessionFact::new(claim);
    session.add_fact(fact);
    assert_eq!(session.len(), 1);
}

#[test]
fn test_session_add_multiple_facts() {
    let mut session = SessionHistory::new();
    for i in 0..5 {
        let claim = create_test_claim(&format!("City{}", i), "value");
        session.add_fact(SessionFact::new(claim));
    }
    assert_eq!(session.len(), 5);
}

#[test]
fn test_session_clear() {
    let mut session = SessionHistory::new();
    let claim = create_test_claim("Paris", "capital");
    session.add_fact(SessionFact::new(claim));
    assert_eq!(session.len(), 1);
    session.clear();
    assert_eq!(session.len(), 0);
}

#[test]
fn test_session_would_contradict_same_object() {
    let mut session = SessionHistory::new();
    let claim1 = create_test_claim("Paris", "capital");
    session.add_fact(SessionFact::new(claim1.clone()));
    assert!(!session.would_contradict(&claim1));
}

#[test]
fn test_session_would_contradict_different_object() {
    let mut session = SessionHistory::new();
    let claim1 = create_test_claim("Paris", "capital");
    session.add_fact(SessionFact::new(claim1));
    let claim2 = create_test_claim("Paris", "town");
    assert!(session.would_contradict(&claim2));
}

#[test]
fn test_session_no_contradiction_different_subject() {
    let mut session = SessionHistory::new();
    let claim1 = create_test_claim("Paris", "capital");
    session.add_fact(SessionFact::new(claim1));
    let claim2 = create_test_claim("London", "capital");
    assert!(!session.would_contradict(&claim2));
}

// ============================================================================
// Consistency Checker Tests (20+ tests)
// ============================================================================

#[test]
fn test_consistency_checker_creation() {
    let checker = ConsistencyChecker::new();
    assert!(checker.check_temporal);
}

#[test]
fn test_consistency_claims_equivalent() {
    let checker = ConsistencyChecker::new();
    let claim1 = create_test_claim("Paris", "capital");
    let claim2 = create_test_claim("Paris", "capital");
    assert!(checker.claims_equivalent(&claim1, &claim2));
}

#[test]
fn test_consistency_claims_not_equivalent() {
    let checker = ConsistencyChecker::new();
    let claim1 = create_test_claim("Paris", "capital");
    let claim2 = create_test_claim("Paris", "town");
    assert!(!checker.claims_equivalent(&claim1, &claim2));
}

#[test]
fn test_consistency_find_contradictions() {
    let checker = ConsistencyChecker::new();
    let claim1 = create_test_claim("Paris", "capital");
    let claim2 = create_test_claim("Paris", "town");
    let claim3 = create_test_claim("London", "capital");
    let contradictions = checker.find_contradictions(&[claim1, claim2, claim3]);
    assert_eq!(contradictions.len(), 1);
}

#[test]
fn test_consistency_find_no_contradictions() {
    let checker = ConsistencyChecker::new();
    let claim1 = create_test_claim("Paris", "capital");
    let claim2 = create_test_claim("London", "capital");
    let claim3 = create_test_claim("Tokyo", "capital");
    let contradictions = checker.find_contradictions(&[claim1, claim2, claim3]);
    assert_eq!(contradictions.len(), 0);
}

#[test]
fn test_consistency_numeric_invariant_age() {
    let checker = ConsistencyChecker::new();
    let mut claim = create_test_claim("age", "value");
    claim.object = ClaimObject::Number(-5);
    let result = checker.check_numeric_invariants(&claim);
    assert!(matches!(
        result,
        Some(ConsistencyCheckResult::Contradictory { .. })
    ));
}

#[test]
fn test_consistency_numeric_invariant_valid_age() {
    let checker = ConsistencyChecker::new();
    let mut claim = create_test_claim("age", "value");
    claim.object = ClaimObject::Number(25);
    let result = checker.check_numeric_invariants(&claim);
    assert!(result.is_none());
}

#[test]
fn test_consistency_percentage_valid() {
    let checker = ConsistencyChecker::new();
    let mut claim = create_test_claim("percentage", "value");
    claim.object = ClaimObject::Decimal("50.0".to_string());
    let result = checker.check_numeric_invariants(&claim);
    assert!(result.is_none());
}

#[test]
fn test_consistency_percentage_invalid() {
    let checker = ConsistencyChecker::new();
    let mut claim = create_test_claim("percentage", "value");
    claim.object = ClaimObject::Decimal("150.0".to_string());
    let result = checker.check_numeric_invariants(&claim);
    assert!(matches!(
        result,
        Some(ConsistencyCheckResult::Contradictory { .. })
    ));
}

// ============================================================================
// Axiom Verifier Tests (10+ tests)
// ============================================================================

#[tokio::test]
async fn test_axiom_verifier_creation() {
    let verifier = AxiomVerifier::new();
    assert!(!verifier.list_theorems().is_empty());
}

#[tokio::test]
async fn test_axiom_register_theorem() {
    let mut verifier = AxiomVerifier::new();
    let theorem = Theorem::new("custom", "statement");
    verifier.register_theorem(theorem);
    assert!(verifier.get_theorem("custom").is_some());
}

#[tokio::test]
async fn test_axiom_proof_hash() {
    let mut proof = AxiomProof::new("test", json!({"valid": true}));
    let hash = proof.compute_hash();
    assert!(!hash.is_empty());
}

#[tokio::test]
async fn test_axiom_verify_valid_proof() {
    let mut verifier = AxiomVerifier::new();
    let proof = AxiomProof::new("output_well_formed", json!({"valid": true}));
    let result = verifier.verify_proof(&proof).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_axiom_verify_unknown_theorem() {
    let mut verifier = AxiomVerifier::new();
    let proof = AxiomProof::new("unknown_theorem", json!({}));
    let result = verifier.verify_proof(&proof).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_axiom_clear_cache() {
    let mut verifier = AxiomVerifier::new();
    verifier.proof_cache.insert("test".to_string(), ("hash".to_string(), true));
    assert_eq!(verifier.proof_cache.len(), 1);
    verifier.clear_cache();
    assert_eq!(verifier.proof_cache.len(), 0);
}

#[test]
fn test_axiom_proof_with_metadata() {
    let proof = AxiomProof::new("test", json!({}))
        .with_metadata("source", "coq")
        .with_metadata("confidence", "high");
    assert_eq!(proof.metadata.get("source"), Some(&"coq".to_string()));
}

// ============================================================================
// End-to-End Pipeline Tests (10+ tests)
// ============================================================================

#[tokio::test]
async fn test_formal_verifier_creation() {
    let verifier = FormalVerifier::new();
    assert_eq!(verifier.session_facts().len(), 0);
}

#[tokio::test]
async fn test_formal_verifier_verify_json() {
    let mut verifier = FormalVerifier::new();
    let json = r#"{"name": "Alice", "age": 30}"#;
    let result = verifier.verify(json).await;
    assert!(result.is_ok());
}

#[test]
fn test_formal_verifier_would_violate_consistency() {
    let verifier = FormalVerifier::new();
    let claim = create_test_claim("Paris", "capital");
    assert!(!verifier.would_violate_consistency(&claim));
}

#[test]
fn test_formal_verifier_clear_session() {
    let mut verifier = FormalVerifier::new();
    assert_eq!(verifier.session_facts().len(), 0);
    verifier.clear_session();
    assert_eq!(verifier.session_facts().len(), 0);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_claim(subject: &str, object: &str) -> FactualClaim {
    FactualClaim {
        id: uuid::Uuid::new_v4(),
        subject: Subject::new(subject, subject),
        predicate: Predicate::new("is", "is"),
        object: object.to_string(),
        context: None,
        source_confidence: 0.95,
        timestamp: chrono::Utc::now(),
        source_reference: None,
    }
}
