use crate::AxiomProof;
use std::collections::HashMap;

pub struct ProofChecker {
    verified_proofs: HashMap<String, AxiomProof>,
}

impl ProofChecker {
    pub fn new() -> Self {
        Self { verified_proofs: HashMap::new() }
    }
    
    pub fn verify(&mut self, proof: &AxiomProof) -> bool {
        for dep in &proof.dependencies {
            if !self.verified_proofs.contains_key(dep) {
                return false;
            }
        }
        let valid = !proof.proof_script.is_empty() && !proof.statement.is_empty();
        if valid {
            let mut verified = proof.clone();
            verified.verified = true;
            self.verified_proofs.insert(proof.name.clone(), verified);
        }
        valid
    }
    
    pub fn is_verified(&self, name: &str) -> bool {
        self.verified_proofs.get(name).map(|p| p.verified).unwrap_or(false)
    }
    
    pub fn get(&self, name: &str) -> Option<&AxiomProof> {
        self.verified_proofs.get(name)
    }
}
