//! Hybrid identity keys: Ed25519 (classical) signing keys.
//!
//! The full TransferDaemon design adds ML-DSA-87 (FIPS 204) as the post-quantum
//! component. We implement the Ed25519 layer now; ML-DSA-87 is additive when the
//! `fips204` crate stabilises (the combined public key would be 2624 bytes).

use crate::error::{CryptoError, CryptoResult};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

// ── Public identity ───────────────────────────────────────────────────────────

/// The public portion of a Bonsai identity — shareable with peers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityPublicKey {
    /// Ed25519 verifying key (32 bytes), hex-encoded for display.
    pub ed25519_pk: [u8; 32],
    /// Fingerprint: first 8 bytes of BLAKE3(ed25519_pk) as hex.
    pub fingerprint: String,
}

impl IdentityPublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        let fp_hash = blake3::hash(&bytes);
        let fingerprint = hex::encode(&fp_hash.as_bytes()[..8]);
        Self {
            ed25519_pk: bytes,
            fingerprint,
        }
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.ed25519_pk)
    }

    pub fn from_hex(s: &str) -> CryptoResult<Self> {
        let bytes = hex::decode(s).map_err(|e| CryptoError::InvalidKey(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKey(
                "Ed25519 key must be 32 bytes".into(),
            ));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self::from_bytes(arr))
    }

    /// Verify an Ed25519 signature over `message`.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> CryptoResult<()> {
        let vk = VerifyingKey::from_bytes(&self.ed25519_pk)
            .map_err(|e| CryptoError::InvalidKey(e.to_string()))?;
        let sig =
            Signature::from_slice(signature).map_err(|e| CryptoError::InvalidKey(e.to_string()))?;
        vk.verify(message, &sig)
            .map_err(|_| CryptoError::HandshakeFailed("signature verification failed".into()))
    }
}

// ── Full identity (private) ───────────────────────────────────────────────────

/// A Bonsai node's full identity (private keys — never serialised to disk directly).
pub struct BonsaiIdentity {
    signing_key: SigningKey,
    pub public_key: IdentityPublicKey,
}

impl BonsaiIdentity {
    /// Generate a fresh random identity.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let pk_bytes = signing_key.verifying_key().to_bytes();
        let public_key = IdentityPublicKey::from_bytes(pk_bytes);
        Self {
            signing_key,
            public_key,
        }
    }

    /// Restore from a raw 32-byte seed (produced by KDF from BIP-39 phrase).
    pub fn from_seed(seed: &[u8; 32]) -> CryptoResult<Self> {
        let signing_key = SigningKey::from_bytes(seed);
        let pk_bytes = signing_key.verifying_key().to_bytes();
        let public_key = IdentityPublicKey::from_bytes(pk_bytes);
        Ok(Self {
            signing_key,
            public_key,
        })
    }

    /// Export the raw 32-byte signing seed (store encrypted, never in plaintext logs).
    pub fn export_seed(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign `message`, returning a 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    /// The public fingerprint for display (e.g., in the UI and QR codes).
    pub fn fingerprint(&self) -> &str {
        &self.public_key.fingerprint
    }
}

impl Drop for BonsaiIdentity {
    fn drop(&mut self) {
        // Zeroize the signing key bytes on drop
        let mut seed = self.signing_key.to_bytes();
        seed.zeroize();
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_sign() {
        let id = BonsaiIdentity::generate();
        let msg = b"hello bonsai";
        let sig = id.sign(msg);
        id.public_key.verify(msg, &sig).unwrap();
    }

    #[test]
    fn hex_roundtrip() {
        let id = BonsaiIdentity::generate();
        let hex = id.public_key.to_hex();
        let recovered = IdentityPublicKey::from_hex(&hex).unwrap();
        assert_eq!(id.public_key.ed25519_pk, recovered.ed25519_pk);
    }

    #[test]
    fn wrong_sig_rejected() {
        let id = BonsaiIdentity::generate();
        let other = BonsaiIdentity::generate();
        let sig = other.sign(b"tampered");
        assert!(id.public_key.verify(b"tampered", &sig).is_err());
    }
}
