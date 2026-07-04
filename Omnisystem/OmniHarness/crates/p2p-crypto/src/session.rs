//! Session key exchange: X25519 ECDH with BLAKE3 key derivation.
//!
//! Protocol (2-message):
//!   Initiator → Responder:  InitiatorHello { x25519_pk }
//!   Responder → Initiator:  ResponderHello { x25519_pk }
//!
//! Session key = BLAKE3.derive_key("bonsai-session-v1", x25519_shared)
//!
//! Note: post-quantum (ML-KEM-768) layer can be added once the ml-kem crate
//! API stabilises. X25519 alone provides 128-bit classical security.

use crate::error::CryptoResult;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use zeroize::Zeroize;

// ── Session key ───────────────────────────────────────────────────────────────

/// A 32-byte session key derived from the ECDH handshake.
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct SessionKey(pub [u8; 32]);

impl SessionKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SessionKey([redacted])")
    }
}

// ── Wire messages ─────────────────────────────────────────────────────────────

/// Sent by the initiator to start the handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiatorHello {
    /// X25519 ephemeral public key (32 bytes).
    pub x25519_pk: [u8; 32],
}

/// Sent by the responder to complete the handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponderHello {
    /// X25519 ephemeral public key (32 bytes).
    pub x25519_pk: [u8; 32],
}

// ── Handshake ─────────────────────────────────────────────────────────────────

pub struct HybridHandshake;

impl HybridHandshake {
    /// **Initiator side**: generate `InitiatorHello` and return the pending state.
    pub fn initiate() -> CryptoResult<(InitiatorHello, InitiatorPending)> {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let pk = X25519PublicKey::from(&secret);
        let hello = InitiatorHello {
            x25519_pk: pk.to_bytes(),
        };
        Ok((hello, InitiatorPending { secret }))
    }

    /// **Responder side**: receive `InitiatorHello`, produce `ResponderHello` + session key.
    pub fn respond(hello: &InitiatorHello) -> CryptoResult<(ResponderHello, SessionKey)> {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let pk = X25519PublicKey::from(&secret);
        let their_pk = X25519PublicKey::from(hello.x25519_pk);
        let shared = secret.diffie_hellman(&their_pk);
        let session_key = derive_session_key(shared.as_bytes());
        Ok((
            ResponderHello {
                x25519_pk: pk.to_bytes(),
            },
            session_key,
        ))
    }
}

/// Holds the initiator's secret while waiting for `ResponderHello`.
pub struct InitiatorPending {
    secret: EphemeralSecret,
}

impl InitiatorPending {
    /// Complete the handshake by processing the responder's reply.
    pub fn complete(self, resp: &ResponderHello) -> CryptoResult<SessionKey> {
        let their_pk = X25519PublicKey::from(resp.x25519_pk);
        let shared = self.secret.diffie_hellman(&their_pk);
        Ok(derive_session_key(shared.as_bytes()))
    }
}

// ── Key derivation ────────────────────────────────────────────────────────────

fn derive_session_key(shared: &[u8]) -> SessionKey {
    let key = blake3::derive_key("bonsai-session-v1", shared);
    SessionKey(key)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_produces_same_key() {
        let (hello, pending) = HybridHandshake::initiate().unwrap();
        let (resp, sk_resp) = HybridHandshake::respond(&hello).unwrap();
        let sk_init = pending.complete(&resp).unwrap();
        assert_eq!(sk_init.0, sk_resp.0, "session keys must match");
    }

    #[test]
    fn different_sessions_produce_different_keys() {
        let (hello1, p1) = HybridHandshake::initiate().unwrap();
        let (hello2, p2) = HybridHandshake::initiate().unwrap();
        let (resp1, sk1) = HybridHandshake::respond(&hello1).unwrap();
        let (resp2, sk2) = HybridHandshake::respond(&hello2).unwrap();
        let _ = p1.complete(&resp1).unwrap();
        let _ = p2.complete(&resp2).unwrap();
        assert_ne!(sk1.0, sk2.0, "different sessions must yield different keys");
    }
}
