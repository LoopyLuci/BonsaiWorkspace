//! BMCS Gateway CLI - runs a query through the classify -> respond -> verify pipeline

use bmcs_gateway::{AxiomVerifier, ContextClassifier, FallbackSystem, InputSanitizer};

fn main() {
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "I'm feeling really anxious about my exam tomorrow".to_string());

    let sanitized = InputSanitizer::sanitize(&query);
    println!("sanitized query: {}", sanitized);
    println!("adversarial: {}", InputSanitizer::is_adversarial(&query));

    let classification = ContextClassifier::classify(&query, None);
    println!(
        "tier: {:?} (confidence {:.2}) - {}",
        classification.tier, classification.confidence, classification.reasoning
    );

    let fallback = FallbackSystem::get_fallback_response(classification.tier);
    println!("fallback response:\n{}", fallback.response);
    println!("disclaimer: {}", fallback.disclaimer);

    let verification = AxiomVerifier::verify_response(&fallback.response);
    println!(
        "axiom check: all_passed={} violations={}",
        verification.all_passed,
        verification.violations.len()
    );
}
