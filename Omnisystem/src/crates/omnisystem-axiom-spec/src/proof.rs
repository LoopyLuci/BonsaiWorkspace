// Proof Engine - manages formal proofs for specifications

use serde::{Deserialize, Serialize};

/// A proof obligation that must be discharged
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ProofObligation {
    InvariantMaintained(String, String), // (invariant_name, module_name)
    PreconditionSatisfied(String, String), // (precondition_name, module_name)
    PostconditionSatisfied(String, String), // (postcondition_name, module_name)
    PropertyProven(String, String), // (property_name, module_name)
    SafetyPropertyHolds(String, String), // (safety_property_name, module_name)
    LivenessPropertyHolds(String, String), // (liveness_property_name, module_name)
}

impl ProofObligation {
    pub fn module_name(&self) -> &str {
        match self {
            ProofObligation::InvariantMaintained(_, m) => m,
            ProofObligation::PreconditionSatisfied(_, m) => m,
            ProofObligation::PostconditionSatisfied(_, m) => m,
            ProofObligation::PropertyProven(_, m) => m,
            ProofObligation::SafetyPropertyHolds(_, m) => m,
            ProofObligation::LivenessPropertyHolds(_, m) => m,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ProofObligation::InvariantMaintained(n, _) => n,
            ProofObligation::PreconditionSatisfied(n, _) => n,
            ProofObligation::PostconditionSatisfied(n, _) => n,
            ProofObligation::PropertyProven(n, _) => n,
            ProofObligation::SafetyPropertyHolds(n, _) => n,
            ProofObligation::LivenessPropertyHolds(n, _) => n,
        }
    }
}

/// A proof that an obligation has been discharged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    pub obligation: ProofObligation,
    pub status: ProofStatus,
    pub strategy: ProofStrategy,
    pub evidence: Vec<ProofEvidence>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Status of a proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStatus {
    Unproven,
    InProgress,
    Proven,
    Discharged,
}

/// Strategy used for proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofStrategy {
    DirectProof(String),           // Direct mathematical proof
    Contradiction(String),         // Proof by contradiction
    Induction(String),            // Inductive proof
    CaseAnalysis(Vec<String>),    // Case-by-case analysis
    ModelChecking(String),        // Automated model checking
    TheoremProving(String),       // Automated theorem proving
    Testing(Vec<String>),         // Comprehensive test suite
}

/// Evidence supporting a proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEvidence {
    pub evidence_type: String,
    pub description: String,
    pub content: String,
}

impl Proof {
    pub fn new(obligation: ProofObligation) -> Self {
        Self {
            obligation,
            status: ProofStatus::Unproven,
            strategy: ProofStrategy::DirectProof("TBD".to_string()),
            evidence: Vec::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn with_strategy(mut self, strategy: ProofStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn add_evidence(mut self, evidence: ProofEvidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub fn mark_proven(mut self) -> Self {
        self.status = ProofStatus::Proven;
        self
    }

    pub fn mark_discharged(mut self) -> Self {
        self.status = ProofStatus::Discharged;
        self
    }
}

/// Proof Engine - manages formal proofs
pub struct ProofEngine {
    proofs: std::collections::HashMap<String, Proof>,
    obligations: Vec<ProofObligation>,
}

impl ProofEngine {
    pub fn new() -> Self {
        Self {
            proofs: std::collections::HashMap::new(),
            obligations: Vec::new(),
        }
    }

    /// Add a proof obligation
    pub fn add_obligation(&mut self, obligation: ProofObligation) {
        self.obligations.push(obligation);
    }

    /// Add multiple obligations
    pub fn add_obligations(&mut self, obligations: Vec<ProofObligation>) {
        self.obligations.extend(obligations);
    }

    /// Create a proof for an obligation
    pub fn prove(&mut self, obligation: ProofObligation, strategy: ProofStrategy) -> Proof {
        let key = format!("{:?}", obligation);
        let mut proof = Proof::new(obligation);
        proof.strategy = strategy;
        self.proofs.insert(key, proof.clone());
        proof
    }

    /// Discharge a proof (mark as verified)
    pub fn discharge(&mut self, obligation: &ProofObligation) -> bool {
        let key = format!("{:?}", obligation);
        if let Some(proof) = self.proofs.get_mut(&key) {
            proof.status = ProofStatus::Discharged;
            true
        } else {
            false
        }
    }

    /// Get proof status for an obligation
    pub fn status(&self, obligation: &ProofObligation) -> ProofStatus {
        let key = format!("{:?}", obligation);
        self.proofs
            .get(&key)
            .map(|p| p.status)
            .unwrap_or(ProofStatus::Unproven)
    }

    /// Get all unproven obligations
    pub fn unproven_obligations(&self) -> Vec<&ProofObligation> {
        self.obligations
            .iter()
            .filter(|ob| {
                self.status(ob) == ProofStatus::Unproven
                    || self.status(ob) == ProofStatus::InProgress
            })
            .collect()
    }

    /// Get proof summary
    pub fn summary(&self) -> ProofSummary {
        let total = self.obligations.len();
        let proven = self
            .proofs
            .values()
            .filter(|p| p.status == ProofStatus::Proven || p.status == ProofStatus::Discharged)
            .count();

        ProofSummary {
            total_obligations: total,
            proven_obligations: proven,
            unproven_obligations: total - proven,
            completion_percentage: if total == 0 {
                100.0
            } else {
                (proven as f64 / total as f64) * 100.0
            },
        }
    }
}

impl Default for ProofEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of proof status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSummary {
    pub total_obligations: usize,
    pub proven_obligations: usize,
    pub unproven_obligations: usize,
    pub completion_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_obligation_module_name() {
        let ob = ProofObligation::InvariantMaintained("inv1".to_string(), "module1".to_string());
        assert_eq!(ob.module_name(), "module1");
        assert_eq!(ob.name(), "inv1");
    }

    #[test]
    fn test_proof_engine() {
        let mut engine = ProofEngine::new();
        let ob = ProofObligation::InvariantMaintained("inv1".to_string(), "module1".to_string());

        engine.add_obligation(ob.clone());
        assert_eq!(engine.status(&ob), ProofStatus::Unproven);

        let proof = engine.prove(ob.clone(), ProofStrategy::DirectProof("test".to_string()));
        assert_eq!(proof.status, ProofStatus::Proven);

        engine.discharge(&ob);
        assert_eq!(engine.status(&ob), ProofStatus::Discharged);
    }

    #[test]
    fn test_proof_summary() {
        let mut engine = ProofEngine::new();
        engine.add_obligation(ProofObligation::InvariantMaintained(
            "inv1".to_string(),
            "module1".to_string(),
        ));
        engine.add_obligation(ProofObligation::InvariantMaintained(
            "inv2".to_string(),
            "module1".to_string(),
        ));

        let summary = engine.summary();
        assert_eq!(summary.total_obligations, 2);
        assert_eq!(summary.proven_obligations, 0);
        assert_eq!(summary.unproven_obligations, 2);
    }
}
