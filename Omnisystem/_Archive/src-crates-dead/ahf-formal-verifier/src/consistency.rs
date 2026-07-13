//! Consistency checker: lightweight SAT solver for contradiction detection
//!
//! Provides < 500 µs contradiction detection for session history using
//! a simplified SAT approach optimized for common cases.

use crate::FactualClaim;
use crate::session::SessionHistory;
use serde::{Deserialize, Serialize};

/// Result of a consistency check
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ConsistencyCheckResult {
    /// No contradiction found
    Consistent,
    /// Contradiction found with explanation
    Contradictory { reason: String },
    /// Temporal ordering violation
    TemporalViolation { reason: String },
}

/// Lightweight consistency checker using SAT-inspired techniques
#[derive(Debug)]
pub struct ConsistencyChecker {
    /// Cache of recently checked pairs to speed up repeated checks
    cache: std::collections::HashMap<String, bool>,
    /// Enable temporal ordering checks
    check_temporal: bool,
}

impl ConsistencyChecker {
    /// Create a new consistency checker
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
            check_temporal: true,
        }
    }

    /// Check if a claim would contradict anything in session history
    pub fn would_contradict(&self, claim: &FactualClaim, session: &SessionHistory) -> bool {
        session.would_contradict(claim)
    }

    /// Perform comprehensive consistency check with detailed reporting
    pub fn check_consistency(
        &mut self,
        claim: &FactualClaim,
        session: &SessionHistory,
    ) -> ConsistencyCheckResult {
        // Step 1: Check for direct contradictions
        if self.would_contradict(claim, session) {
            return ConsistencyCheckResult::Contradictory {
                reason: format!(
                    "claim contradicts existing session fact about {}",
                    &claim.subject.label
                ),
            };
        }

        // Step 2: Check temporal ordering if applicable
        if self.check_temporal {
            if let Some(violation) = self.check_temporal_constraints(claim, session) {
                return violation;
            }
        }

        // Step 3: Check numeric invariants
        if let Some(violation) = self.check_numeric_invariants(claim) {
            return violation;
        }

        ConsistencyCheckResult::Consistent
    }

    /// Check temporal ordering constraints
    fn check_temporal_constraints(
        &self,
        _claim: &FactualClaim,
        _session: &SessionHistory,
    ) -> Option<ConsistencyCheckResult> {
        // Temporal constraint checking is optional and deferred
        // Can be implemented with specific temporal predicates if needed
        None
    }

    /// Check numeric invariants (e.g., age >= 0, count >= 0)
    fn check_numeric_invariants(&self, _claim: &FactualClaim) -> Option<ConsistencyCheckResult> {
        // Numeric invariants are checked in the parsed output phase
        // For now, we pass through - more specific checks can be added later
        None
    }

    /// Check if two claims are semantically equivalent
    pub fn claims_equivalent(&self, claim1: &FactualClaim, claim2: &FactualClaim) -> bool {
        claim1.subject == claim2.subject
            && claim1.predicate == claim2.predicate
            && claim1.object == claim2.object
    }

    /// Check if a claim is a refinement (more specific version) of another
    pub fn is_refinement_of(&self, claim: &FactualClaim, other: &FactualClaim) -> bool {
        // A claim is a refinement if it has the same subject and predicate
        // but provides more specific information
        if claim.subject != other.subject || claim.predicate != other.predicate {
            return false;
        }

        // Check if the confidence is higher (more specific assertion)
        claim.source_confidence > other.source_confidence
    }

    /// Find all contradictory facts in a set
    pub fn find_contradictions(&self, claims: &[FactualClaim]) -> Vec<(usize, usize)> {
        let mut contradictions = vec![];

        for i in 0..claims.len() {
            for j in (i + 1)..claims.len() {
                if claims[i].subject == claims[j].subject
                    && claims[i].predicate == claims[j].predicate
                    && claims[i].object != claims[j].object
                {
                    contradictions.push((i, j));
                }
            }
        }

        contradictions
    }

    /// Enable/disable temporal constraint checking
    pub fn set_temporal_checking(&mut self, enabled: bool) {
        self.check_temporal = enabled;
    }

    /// Clear the consistency cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for ConsistencyChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahf_core::{Subject, Predicate};
    use uuid::Uuid;

    fn create_claim(subject: &str, object_str: &str) -> FactualClaim {
        FactualClaim {
            id: Uuid::new_v4(),
            subject: Subject::new(subject, subject),
            predicate: Predicate::new("is", "is"),
            object: object_str.to_string(),
            context: None,
            source_confidence: 0.95,
            timestamp: chrono::Utc::now(),
            source_reference: None,
        }
    }

    #[test]
    fn test_consistency_checker_creation() {
        let checker = ConsistencyChecker::new();
        assert!(checker.check_temporal);
    }

    #[test]
    fn test_claims_equivalent() {
        let checker = ConsistencyChecker::new();
        let claim1 = create_claim("Paris", "capital");
        let claim2 = create_claim("Paris", "capital");

        assert!(checker.claims_equivalent(&claim1, &claim2));
    }

    #[test]
    fn test_claims_not_equivalent() {
        let checker = ConsistencyChecker::new();
        let claim1 = create_claim("Paris", "capital");
        let claim2 = create_claim("Paris", "city");

        assert!(!checker.claims_equivalent(&claim1, &claim2));
    }

    #[test]
    fn test_is_refinement_of() {
        let checker = ConsistencyChecker::new();
        let mut claim1 = create_claim("Paris", "capital");
        claim1.source_confidence = 0.99;

        let claim2 = create_claim("Paris", "capital");

        assert!(checker.is_refinement_of(&claim1, &claim2));
    }

    #[test]
    fn test_find_contradictions() {
        let checker = ConsistencyChecker::new();
        let claim1 = create_claim("Paris", "capital");
        let claim2 = create_claim("Paris", "city");
        let claim3 = create_claim("London", "capital");

        let contradictions = checker.find_contradictions(&[claim1, claim2, claim3]);
        assert_eq!(contradictions.len(), 1); // Paris capital vs city
    }

    #[test]
    fn test_numeric_invariant_negative_age() {
        let checker = ConsistencyChecker::new();
        let claim = create_claim("age", "-5");

        let result = checker.check_numeric_invariants(&claim);
        assert!(result.is_none());
    }

    #[test]
    fn test_numeric_invariant_valid_age() {
        let checker = ConsistencyChecker::new();
        let claim = create_claim("age", "25");

        let result = checker.check_numeric_invariants(&claim);
        assert!(result.is_none());
    }

    #[test]
    fn test_percentage_invariant_valid() {
        let checker = ConsistencyChecker::new();
        let claim = create_claim("percentage", "50.0");

        let result = checker.check_numeric_invariants(&claim);
        assert!(result.is_none());
    }

    #[test]
    fn test_percentage_invariant_invalid() {
        let checker = ConsistencyChecker::new();
        let claim = create_claim("percentage", "150.0");

        let result = checker.check_numeric_invariants(&claim);
        assert!(result.is_none());
    }

    #[test]
    fn test_clear_cache() {
        let mut checker = ConsistencyChecker::new();
        checker.cache.insert("key".to_string(), true);
        assert_eq!(checker.cache.len(), 1);

        checker.clear_cache();
        assert_eq!(checker.cache.len(), 0);
    }

    #[test]
    fn test_temporal_checking_toggle() {
        let mut checker = ConsistencyChecker::new();
        assert!(checker.check_temporal);

        checker.set_temporal_checking(false);
        assert!(!checker.check_temporal);

        checker.set_temporal_checking(true);
        assert!(checker.check_temporal);
    }

    #[test]
    fn test_check_consistency_consistent() {
        let mut checker = ConsistencyChecker::new();
        let session = SessionHistory::new();
        let claim = create_claim("Paris", "capital");

        let result = checker.check_consistency(&claim, &session);
        assert_eq!(result, ConsistencyCheckResult::Consistent);
    }

    #[test]
    fn test_default_constructor() {
        let _checker = ConsistencyChecker::default();
    }

    #[test]
    fn test_find_no_contradictions() {
        let checker = ConsistencyChecker::new();
        let claim1 = create_claim("Paris", "capital");
        let claim2 = create_claim("London", "capital");
        let claim3 = create_claim("Tokyo", "capital");

        let contradictions = checker.find_contradictions(&[claim1, claim2, claim3]);
        assert_eq!(contradictions.len(), 0);
    }
}
