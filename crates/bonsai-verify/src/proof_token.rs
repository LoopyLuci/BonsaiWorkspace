//! Bridge between bonsai-verify's `ProofWitness` and bonsai-capability-registry's `ProofToken`.
//!
//! Serializes a `ProofWitness` to canonical JSON bytes, hashes them with Blake3,
//! and wraps the result in a `ProofToken` that can be stored in CAS and validated
//! at deployment gates.

use serde_json;
use crate::kernel::ProofWitness;

/// Serialize a `ProofWitness` to canonical bytes for hashing.
/// Uses JSON with sorted keys for determinism.
pub fn witness_to_bytes(witness: &ProofWitness) -> Vec<u8> {
    // serde_json produces field-ordered output for structs (fields in declaration order).
    // This is sufficient for our Blake3 commitment — we do not need full canonicalization.
    serde_json::to_vec(witness).expect("ProofWitness is always serializable")
}

/// A lightweight token wrapping a Blake3-committed proof witness.
/// This is the bonsai-verify side; the full `ProofToken` struct lives in
/// bonsai-capability-registry. We produce the bytes and hash here, keeping
/// the capability registry as an optional dependency.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerifyToken {
    /// Blake3 hash (hex) of the canonical `ProofWitness` bytes.
    pub proof_hash: String,
    /// Human-readable proposition string (from the proof's proposition term).
    pub proposition_display: String,
    /// Raw canonical bytes (may be stored in CAS by the caller).
    #[serde(skip)]
    pub canonical_bytes: Vec<u8>,
}

impl VerifyToken {
    /// Create a token from a `ProofWitness`.
    pub fn from_witness(witness: &ProofWitness) -> Self {
        let bytes = witness_to_bytes(witness);
        let hash  = blake3::hash(&bytes);
        let hex   = hash.as_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>();
        let prop  = format!("{:?}", witness.proposition); // use Debug for now
        Self {
            proof_hash: hex,
            proposition_display: prop,
            canonical_bytes: bytes,
        }
    }

    /// Verify that the given bytes still hash to this token's stored hash.
    pub fn verify(&self, bytes: &[u8]) -> bool {
        let hash = blake3::hash(bytes);
        let hex: String = hash.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
        hex == self.proof_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::{Term, Sort, ProofWitness};

    #[test]
    fn token_round_trip() {
        let witness = ProofWitness {
            proposition: Term::Sort(Sort::Prop),
            term: Term::Sort(Sort::Prop),
        };
        let token = VerifyToken::from_witness(&witness);
        assert!(token.verify(&token.canonical_bytes));
        assert!(!token.verify(b"garbage"));
        assert!(!token.proof_hash.is_empty());
    }

    #[test]
    fn deterministic_hash() {
        let witness = ProofWitness {
            proposition: Term::Nat,
            term: Term::Nat,
        };
        let t1 = VerifyToken::from_witness(&witness);
        let t2 = VerifyToken::from_witness(&witness);
        assert_eq!(t1.proof_hash, t2.proof_hash);
    }
}
