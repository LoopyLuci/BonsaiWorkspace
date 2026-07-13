//! Self-certifying NodeId – the peer's identity IS its public key

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use serde::{Serialize, Deserialize};
use std::fmt;

/// A self-certifying NodeId — the peer's identity IS its Ed25519 public key.
/// No separate certificate authority needed; fully sovereign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    /// Generate a new random identity and its secret key.
    pub fn generate() -> (Self, SigningKey) {
        let mut csprng = OsRng;
        let secret = SigningKey::generate(&mut csprng);
        let public = secret.verifying_key();
        (Self(public.to_bytes()), secret)
    }

    /// Prove ownership of this identity by signing a challenge.
    pub fn prove(&self, secret: &SigningKey, challenge: &[u8]) -> Vec<u8> {
        secret.sign(challenge).to_bytes().to_vec()
    }

    /// Verify that a proof was produced by the owner of this NodeId.
    pub fn verify_proof(&self, challenge: &[u8], signature_bytes: &[u8]) -> bool {
        if let Ok(pk) = VerifyingKey::from_bytes(&self.0) {
            if let Ok(sig) = Signature::from_slice(signature_bytes) {
                return pk.verify(challenge, &sig).is_ok();
            }
        }
        false
    }

    /// Convert from raw bytes.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }

    /// Return as hex string (short form, first 8 bytes).
    pub fn short_hex(&self) -> String {
        hex::encode(&self.0[..8])
    }

    /// Return as full hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.short_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_prove() {
        let (id, secret) = NodeId::generate();
        let challenge = b"hello transfer daemon v2";
        let proof = id.prove(&secret, challenge);
        assert!(id.verify_proof(challenge, &proof));
        assert!(!id.verify_proof(challenge, b"bad signature"));
    }

    #[test]
    fn test_node_id_roundtrip() {
        let (id, _) = NodeId::generate();
        let bytes = id.0;
        let id2 = NodeId::from_bytes(&bytes);
        assert_eq!(id, id2);
    }
}
