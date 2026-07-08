//! Session history and fact tracking for consistency checking
//!
//! Maintains a record of accepted facts in the current session and provides
//! rapid contradiction detection for new claims.

use crate::FactualClaim;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A fact accepted in the current session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFact {
    /// The claim itself
    pub claim: FactualClaim,
    /// When the fact was accepted
    pub timestamp: DateTime<Utc>,
    /// Whether this fact was formally verified
    pub verified: bool,
}

impl SessionFact {
    /// Create a new session fact
    pub fn new(claim: FactualClaim) -> Self {
        Self {
            claim,
            timestamp: Utc::now(),
            verified: false,
        }
    }

    /// Mark fact as verified
    pub fn verified(mut self) -> Self {
        self.verified = true;
        self
    }
}

/// Session history: all facts accepted in the current session
///
/// Provides O(1) lookup for contradiction detection with < 500 µs
/// worst-case performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHistory {
    /// All facts in insertion order
    facts: Vec<SessionFact>,
    /// Quick lookup index: (subject, predicate) -> fact indices
    index: std::collections::HashMap<(String, String), Vec<usize>>,
}

impl SessionHistory {
    /// Create a new empty session history
    pub fn new() -> Self {
        Self {
            facts: Vec::new(),
            index: std::collections::HashMap::new(),
        }
    }

    /// Add a fact to the session
    pub fn add_fact(&mut self, fact: SessionFact) {
        let subject_pred_key = (
            fact.claim.subject.id.clone(),
            fact.claim.predicate.id.clone(),
        );

        let index = self.facts.len();
        self.facts.push(fact);

        self.index
            .entry(subject_pred_key)
            .or_insert_with(Vec::new)
            .push(index);
    }

    /// Get all facts related to a subject and predicate
    pub fn get_facts_by_subject_predicate(
        &self,
        subject: &str,
        predicate: &str,
    ) -> Vec<&SessionFact> {
        let key = (subject.to_string(), predicate.to_string());
        self.index
            .get(&key)
            .map(|indices| indices.iter().map(|&i| &self.facts[i]).collect())
            .unwrap_or_default()
    }

    /// Get all facts in the session
    pub fn facts(&self) -> Vec<SessionFact> {
        self.facts.clone()
    }

    /// Check if the session is empty
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Number of facts in the session
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    /// Clear all facts (useful for testing)
    pub fn clear(&mut self) {
        self.facts.clear();
        self.index.clear();
    }

    /// Get facts that might contradict a given claim
    pub fn get_potential_contradictions(&self, claim: &FactualClaim) -> Vec<&SessionFact> {
        self.get_facts_by_subject_predicate(&claim.subject.id, &claim.predicate.id)
    }

    /// Check if a claim would contradict any existing facts
    pub fn would_contradict(&self, claim: &FactualClaim) -> bool {
        let potentials = self.get_potential_contradictions(claim);
        potentials.iter().any(|fact| fact.claim.object != claim.object)
    }

    /// Get all facts with a specific object
    pub fn get_facts_by_object(&self, object: &str) -> Vec<&SessionFact> {
        self.facts
            .iter()
            .filter(|fact| fact.claim.object == object)
            .collect()
    }
}

impl Default for SessionHistory {
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
    fn test_session_history_creation() {
        let session = SessionHistory::new();
        assert!(session.is_empty());
        assert_eq!(session.len(), 0);
    }

    #[test]
    fn test_add_fact() {
        let mut session = SessionHistory::new();
        let claim = create_claim("Paris", "capital");
        let fact = SessionFact::new(claim);
        session.add_fact(fact);

        assert_eq!(session.len(), 1);
        assert!(!session.is_empty());
    }

    #[test]
    fn test_add_multiple_facts() {
        let mut session = SessionHistory::new();
        for i in 0..10 {
            let claim = create_claim(&format!("Entity{}", i), "value");
            let fact = SessionFact::new(claim);
            session.add_fact(fact);
        }

        assert_eq!(session.len(), 10);
    }

    #[test]
    fn test_get_facts_by_subject_predicate() {
        let mut session = SessionHistory::new();
        let claim1 = create_claim("Paris", "capital");
        let claim2 = create_claim("Paris", "city");

        session.add_fact(SessionFact::new(claim1));
        session.add_fact(SessionFact::new(claim2.clone()));

        let facts = session.get_facts_by_subject_predicate("Paris", "Is");
        assert_eq!(facts.len(), 2);
    }

    #[test]
    fn test_would_contradict_same_object() {
        let mut session = SessionHistory::new();
        let claim1 = create_claim("Paris", "capital");
        session.add_fact(SessionFact::new(claim1.clone()));

        // Same claim should not contradict
        assert!(!session.would_contradict(&claim1));
    }

    #[test]
    fn test_would_contradict_different_object() {
        let mut session = SessionHistory::new();
        let claim1 = create_claim("Paris", "capital");
        session.add_fact(SessionFact::new(claim1));

        let claim2 = create_claim("Paris", "town");
        // Different object should contradict
        assert!(session.would_contradict(&claim2));
    }

    #[test]
    fn test_get_potential_contradictions() {
        let mut session = SessionHistory::new();
        let claim1 = create_claim("Paris", "capital");
        session.add_fact(SessionFact::new(claim1.clone()));

        let potentials = session.get_potential_contradictions(&claim1);
        assert_eq!(potentials.len(), 1);
    }

    #[test]
    fn test_no_contradiction_different_subject() {
        let mut session = SessionHistory::new();
        let claim1 = create_claim("Paris", "capital");
        session.add_fact(SessionFact::new(claim1));

        let claim2 = create_claim("London", "capital");
        assert!(!session.would_contradict(&claim2));
    }

    #[test]
    fn test_clear_session() {
        let mut session = SessionHistory::new();
        let claim = create_claim("Paris", "capital");
        session.add_fact(SessionFact::new(claim));

        assert_eq!(session.len(), 1);
        session.clear();
        assert_eq!(session.len(), 0);
    }

    #[test]
    fn test_session_fact_verified() {
        let claim = create_claim("Paris", "capital");
        let fact = SessionFact::new(claim).verified();
        assert!(fact.verified);
    }

    #[test]
    fn test_get_facts_by_object() {
        let mut session = SessionHistory::new();
        let claim = create_claim("Paris", "capital");
        session.add_fact(SessionFact::new(claim));

        let facts = session.get_facts_by_object("capital");
        assert_eq!(facts.len(), 1);
    }

    #[test]
    fn test_get_facts_by_nonexistent_object() {
        let session = SessionHistory::new();
        let facts = session.get_facts_by_object("nonexistent");
        assert_eq!(facts.len(), 0);
    }

    #[test]
    fn test_default_constructor() {
        let session = SessionHistory::default();
        assert!(session.is_empty());
    }

    #[test]
    fn test_multiple_subjects_same_predicate() {
        let mut session = SessionHistory::new();
        let claim1 = create_claim("Paris", "capital");
        let claim2 = create_claim("London", "capital");

        session.add_fact(SessionFact::new(claim1));
        session.add_fact(SessionFact::new(claim2));

        let paris_facts = session.get_facts_by_subject_predicate("Paris", "Is");
        let london_facts = session.get_facts_by_subject_predicate("London", "Is");

        assert_eq!(paris_facts.len(), 1);
        assert_eq!(london_facts.len(), 1);
    }
}
