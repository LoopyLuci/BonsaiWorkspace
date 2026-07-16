//! Verifiable Credentials (VCs) for group membership and attribute proofs

use serde::{Serialize, Deserialize};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use chrono::{DateTime, Utc};

/// A Verifiable Credential — proves claims without revealing identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    pub issuer: String,
    pub subject: String,
    pub claims: serde_json::Value,
    pub issuance_date: DateTime<Utc>,
    pub expiration_date: Option<DateTime<Utc>>,
    pub proof: CredentialProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialProof {
    pub proof_type: String,
    pub created: DateTime<Utc>,
    pub verification_method: String,
    pub signature_value: Vec<u8>,
}

impl VerifiableCredential {
    /// Verify the signature on this credential using the issuer's DID document.
    pub fn verify(&self, issuer_did: &crate::DidDocument) -> bool {
        let vm = match issuer_did.find_method(&self.proof.verification_method) {
            Some(v) => v,
            None => return false,
        };

        let pk = match VerifyingKey::from_bytes(&vm.public_key_bytes) {
            Ok(k) => k,
            Err(_) => return false,
        };

        let sig = match Signature::from_slice(&self.proof.signature_value) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let payload = self.to_canonical_json();
        pk.verify(payload.as_bytes(), &sig).is_ok()
    }

    /// Check if the credential is currently valid.
    pub fn is_valid_now(&self) -> bool {
        let now = Utc::now();
        now >= self.issuance_date && self.expiration_date.map_or(true, |e| now <= e)
    }

    /// Serialize to canonical JSON for signature verification.
    fn to_canonical_json(&self) -> String {
        serde_json::to_string(&serde_json::json!({
            "issuer": self.issuer,
            "subject": self.subject,
            "claims": self.claims,
            "issuanceDate": self.issuance_date.to_rfc3339(),
            "expirationDate": self.expiration_date.map(|d| d.to_rfc3339()),
        }))
        .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vc_validity() {
        let now = Utc::now();
        let vc = VerifiableCredential {
            issuer: "did:key:abc".into(),
            subject: "did:key:def".into(),
            claims: serde_json::json!({"role": "developer"}),
            issuance_date: now,
            expiration_date: Some(now + chrono::Duration::hours(1)),
            proof: CredentialProof {
                proof_type: "Ed25519Signature2020".into(),
                created: now,
                verification_method: "did:key:abc#keys-1".into(),
                signature_value: vec![],
            },
        };
        assert!(vc.is_valid_now());
    }
}
