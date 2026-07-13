//! Axiom management and validation

use crate::Axiom;

/// Validates axiom consistency
pub fn validate_axiom(axiom: &Axiom) -> bool {
    !axiom.id.is_empty()
        && !axiom.triple.subject.is_empty()
        && !axiom.triple.predicate.is_empty()
        && axiom.confidence >= 0.0
        && axiom.confidence <= 1.0
}
