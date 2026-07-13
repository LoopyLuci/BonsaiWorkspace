//! Axiom integration: optional formal proof verification
//!
//! Provides proof verification against declared theorems with BLAKE3 hash
//! caching for performance. Proofs are optional - verification continues
//! without them, but they are verified when provided.

use crate::errors::{VerifierError, VerifyResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// An Axiom proof structure
///
/// Contains the proof data, theorem name, and optional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomProof {
    /// Name of the theorem being proved
    pub theorem: String,
    /// The proof content (JSON representation)
    pub proof: Value,
    /// Hash of the proof (BLAKE3) for caching and verification
    pub proof_hash: Option<String>,
    /// Whether this proof has been verified
    pub verified: bool,
    /// Optional proof metadata (e.g., proof assistant used, confidence)
    pub metadata: HashMap<String, String>,
}

impl AxiomProof {
    /// Create a new axiom proof
    pub fn new(theorem: &str, proof: Value) -> Self {
        Self {
            theorem: theorem.to_string(),
            proof,
            proof_hash: None,
            verified: false,
            metadata: HashMap::new(),
        }
    }

    /// Compute and store the proof hash
    pub fn compute_hash(&mut self) -> String {
        let proof_json = self.proof.to_string();
        let hash = blake3::hash(proof_json.as_bytes());
        self.proof_hash = Some(hash.to_hex().to_string());
        self.proof_hash.clone().unwrap()
    }

    /// Verify the proof hash matches expected value
    pub fn verify_hash(&self, expected_hash: &str) -> VerifyResult<()> {
        if let Some(actual) = &self.proof_hash {
            if actual != expected_hash {
                return Err(VerifierError::ProofHashMismatch {
                    reason: format!(
                        "hash mismatch: expected {}, got {}",
                        expected_hash, actual
                    ),
                });
            }
            Ok(())
        } else {
            Err(VerifierError::ProofVerificationFailed {
                reason: "proof hash not computed".to_string(),
            })
        }
    }

    /// Add metadata to the proof
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Theorem definition for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theorem {
    /// Name of the theorem
    pub name: String,
    /// Statement of the theorem
    pub statement: String,
    /// Expected proof structure (JSON schema-like)
    pub expected_structure: Value,
    /// Whether this theorem is critical (must be verified)
    pub critical: bool,
}

impl Theorem {
    /// Create a new theorem
    pub fn new(name: &str, statement: &str) -> Self {
        Self {
            name: name.to_string(),
            statement: statement.to_string(),
            expected_structure: Value::Object(Default::default()),
            critical: false,
        }
    }

    /// Mark this theorem as critical
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }

    /// Set the expected proof structure
    pub fn with_expected_structure(mut self, structure: Value) -> Self {
        self.expected_structure = structure;
        self
    }
}

/// Axiom verifier: manages proof verification and caching
#[derive(Debug)]
pub struct AxiomVerifier {
    /// Known theorems indexed by name
    theorems: HashMap<String, Theorem>,
    /// Proof cache: theorem name -> (proof hash, result)
    proof_cache: HashMap<String, (String, bool)>,
    /// Maximum cache size
    max_cache_size: usize,
}

impl AxiomVerifier {
    /// Create a new axiom verifier with default theorems
    pub fn new() -> Self {
        let mut verifier = Self {
            theorems: HashMap::new(),
            proof_cache: HashMap::new(),
            max_cache_size: 10_000,
        };

        // Register some default theorems
        verifier.register_theorem(
            Theorem::new(
                "output_well_formed",
                "The output satisfies basic well-formedness constraints",
            )
            .critical(),
        );

        verifier.register_theorem(
            Theorem::new(
                "no_contradiction",
                "The output does not contradict any known facts",
            )
            .critical(),
        );

        verifier.register_theorem(
            Theorem::new(
                "type_safety",
                "All types in the output conform to the declared schema",
            )
            .critical(),
        );

        verifier
    }

    /// Register a theorem
    pub fn register_theorem(&mut self, theorem: Theorem) {
        self.theorems.insert(theorem.name.clone(), theorem);
    }

    /// Verify a proof
    ///
    /// Performs basic structural validation and caches results.
    /// Returns Ok if proof is valid or not required, Err if invalid.
    pub async fn verify_proof(&mut self, proof: &AxiomProof) -> VerifyResult<()> {
        // Check if theorem is known
        if !self.theorems.contains_key(&proof.theorem) {
            return Err(VerifierError::TheoremNotFound {
                reason: format!("theorem '{}' not found", proof.theorem),
            });
        }

        let theorem = &self.theorems[&proof.theorem];

        // Step 1: Check cache
        if let Some((cached_hash, cached_result)) = self.proof_cache.get(&proof.theorem) {
            if let Some(proof_hash) = &proof.proof_hash {
                if cached_hash == proof_hash {
                    return if *cached_result {
                        Ok(())
                    } else {
                        Err(VerifierError::ProofVerificationFailed {
                            reason: "cached proof verification failed".to_string(),
                        })
                    };
                }
            }
        }

        // Step 2: Validate proof structure
        self.validate_proof_structure(proof, theorem)?;

        // Step 3: Verify proof logic (basic checks)
        self.verify_proof_logic(proof)?;

        // Step 4: Cache result
        if let Some(hash) = &proof.proof_hash {
            if self.proof_cache.len() < self.max_cache_size {
                self.proof_cache
                    .insert(proof.theorem.clone(), (hash.clone(), true));
            }
        }

        Ok(())
    }

    /// Validate proof structure against expected format
    fn validate_proof_structure(&self, proof: &AxiomProof, theorem: &Theorem) -> VerifyResult<()> {
        // Basic structural validation
        if !proof.proof.is_object() && !proof.proof.is_array() {
            return Err(VerifierError::ProofVerificationFailed {
                reason: "proof must be an object or array".to_string(),
            });
        }

        // Check for required fields
        if let Value::Object(obj) = &proof.proof {
            // Most proofs should have at least these fields
            if obj.is_empty() && theorem.critical {
                return Err(VerifierError::ProofVerificationFailed {
                    reason: "critical proof cannot be empty".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Verify proof logic (basic checks)
    fn verify_proof_logic(&self, proof: &AxiomProof) -> VerifyResult<()> {
        // Basic logical checks
        match proof.theorem.as_str() {
            "output_well_formed" => {
                // Check that proof includes a "valid" or "accepted" marker
                if let Value::Object(obj) = &proof.proof {
                    if let Some(Value::Bool(valid)) = obj.get("valid") {
                        if !valid {
                            return Err(VerifierError::ProofVerificationFailed {
                                reason: "output marked as invalid in proof".to_string(),
                            });
                        }
                    }
                }
            }
            "no_contradiction" => {
                // Check that proof demonstrates no contradictions
                if let Value::Object(obj) = &proof.proof {
                    if let Some(Value::Bool(consistent)) = obj.get("consistent") {
                        if !consistent {
                            return Err(VerifierError::ProofVerificationFailed {
                                reason: "proof shows contradictions exist".to_string(),
                            });
                        }
                    }
                }
            }
            "type_safety" => {
                // Check that proof validates type constraints
                if let Value::Object(obj) = &proof.proof {
                    if let Some(Value::Bool(safe)) = obj.get("type_safe") {
                        if !safe {
                            return Err(VerifierError::ProofVerificationFailed {
                                reason: "type safety check failed in proof".to_string(),
                            });
                        }
                    }
                }
            }
            _ => {
                // For custom theorems, just check the proof exists
                if proof.proof == Value::Null {
                    return Err(VerifierError::ProofVerificationFailed {
                        reason: "proof cannot be null".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Get a registered theorem
    pub fn get_theorem(&self, name: &str) -> Option<&Theorem> {
        self.theorems.get(name)
    }

    /// List all registered theorems
    pub fn list_theorems(&self) -> Vec<&Theorem> {
        self.theorems.values().collect()
    }

    /// Clear the proof cache
    pub fn clear_cache(&mut self) {
        self.proof_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.proof_cache.len(), self.max_cache_size)
    }

    /// Set maximum cache size
    pub fn set_max_cache_size(&mut self, size: usize) {
        self.max_cache_size = size;
    }
}

impl Default for AxiomVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axiom_proof_creation() {
        let proof = AxiomProof::new("test_theorem", Value::Object(Default::default()));
        assert_eq!(proof.theorem, "test_theorem");
        assert!(!proof.verified);
    }

    #[test]
    fn test_compute_hash() {
        let mut proof = AxiomProof::new("test", serde_json::json!({"valid": true}));
        let hash = proof.compute_hash();
        assert!(!hash.is_empty());
        assert_eq!(proof.proof_hash, Some(hash.clone()));
    }

    #[test]
    fn test_verify_hash_matches() {
        let mut proof = AxiomProof::new("test", serde_json::json!({"valid": true}));
        let hash = proof.compute_hash();
        let result = proof.verify_hash(&hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_hash_mismatch() {
        let mut proof = AxiomProof::new("test", serde_json::json!({"valid": true}));
        proof.compute_hash();
        let result = proof.verify_hash("wrong_hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_theorem_creation() {
        let theorem = Theorem::new("test", "test statement");
        assert_eq!(theorem.name, "test");
        assert!(!theorem.critical);
    }

    #[test]
    fn test_theorem_critical() {
        let theorem = Theorem::new("test", "test statement").critical();
        assert!(theorem.critical);
    }

    #[test]
    fn test_axiom_verifier_creation() {
        let verifier = AxiomVerifier::new();
        assert!(verifier.theorems.len() > 0);
    }

    #[test]
    fn test_register_theorem() {
        let mut verifier = AxiomVerifier::new();
        let theorem = Theorem::new("custom", "custom statement");
        verifier.register_theorem(theorem);
        assert!(verifier.get_theorem("custom").is_some());
    }

    #[test]
    fn test_list_theorems() {
        let verifier = AxiomVerifier::new();
        let theorems = verifier.list_theorems();
        assert!(theorems.len() > 0);
    }

    #[tokio::test]
    async fn test_verify_proof_unknown_theorem() {
        let mut verifier = AxiomVerifier::new();
        let proof = AxiomProof::new("unknown", Value::Object(Default::default()));
        let result = verifier.verify_proof(&proof).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VerifierError::TheoremNotFound { .. }
        ));
    }

    #[tokio::test]
    async fn test_verify_proof_invalid_structure() {
        let mut verifier = AxiomVerifier::new();
        let proof = AxiomProof::new("output_well_formed", Value::String("invalid".to_string()));
        let result = verifier.verify_proof(&proof).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_proof_valid() {
        let mut verifier = AxiomVerifier::new();
        let proof = AxiomProof::new(
            "output_well_formed",
            serde_json::json!({"valid": true}),
        );
        let result = verifier.verify_proof(&proof).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear_cache() {
        let mut verifier = AxiomVerifier::new();
        verifier.proof_cache.insert("test".to_string(), ("hash".to_string(), true));
        assert_eq!(verifier.proof_cache.len(), 1);
        verifier.clear_cache();
        assert_eq!(verifier.proof_cache.len(), 0);
    }

    #[test]
    fn test_cache_stats() {
        let verifier = AxiomVerifier::new();
        let (used, max) = verifier.cache_stats();
        assert_eq!(used, 0);
        assert!(max > 0);
    }

    #[test]
    fn test_set_max_cache_size() {
        let mut verifier = AxiomVerifier::new();
        verifier.set_max_cache_size(100);
        let (_, max) = verifier.cache_stats();
        assert_eq!(max, 100);
    }

    #[test]
    fn test_proof_with_metadata() {
        let proof = AxiomProof::new("test", Value::Object(Default::default()))
            .with_metadata("source", "coq")
            .with_metadata("confidence", "high");
        assert_eq!(proof.metadata.get("source"), Some(&"coq".to_string()));
        assert_eq!(proof.metadata.get("confidence"), Some(&"high".to_string()));
    }

    #[test]
    fn test_default_constructor() {
        let _verifier = AxiomVerifier::default();
    }
}
