//! Relay session tokens — BLAKE3-derived, proof-of-work protected.
//!
//! A token is 32 bytes derived from the session key and relay ID:
//!   `blake3::derive_key("bonsai-relay-token-v1", session_key || relay_id)`
//!
//! Proof-of-work: the SHA3 of `token || nonce` must start with POW_BITS zero bits.
//! This rate-limits relay slot allocation to ~1 ms per attempt.

use rand::RngCore;
use serde::{Deserialize, Serialize};

/// Required leading zero bits for the PoW nonce.
const POW_BITS: u32 = 16;

/// A 32-byte relay session token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelayToken(pub [u8; 32]);

impl RelayToken {
    /// Derive a token from `session_key` and `relay_id`.
    pub fn derive(session_key: &[u8; 32], relay_id: &[u8]) -> Self {
        let mut input = Vec::with_capacity(32 + relay_id.len());
        input.extend_from_slice(session_key);
        input.extend_from_slice(relay_id);
        let hash = blake3::derive_key("bonsai-relay-token-v1", &input);
        RelayToken(hash)
    }

    /// Mint a random token (for relay-server-assigned sessions).
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        RelayToken(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(RelayToken(arr))
    }
}

/// Proof-of-work nonce + token presented during relay registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub token: RelayToken,
    /// Nonce such that BLAKE3(token || nonce)[..] has POW_BITS leading zeros.
    pub pow_nonce: u64,
}

impl RegisterRequest {
    /// Mine a valid PoW nonce for `token`. Blocks the caller.
    pub fn mine(token: RelayToken) -> Self {
        let mut nonce: u64 = 0;
        loop {
            if check_pow(&token, nonce) {
                return Self {
                    token,
                    pow_nonce: nonce,
                };
            }
            nonce = nonce.wrapping_add(1);
        }
    }

    pub fn verify(&self) -> bool {
        check_pow(&self.token, self.pow_nonce)
    }
}

fn check_pow(token: &RelayToken, nonce: u64) -> bool {
    let mut input = [0u8; 40];
    input[..32].copy_from_slice(&token.0);
    input[32..].copy_from_slice(&nonce.to_be_bytes());
    let hash = blake3::hash(&input);
    let h = hash.as_bytes();
    // Check POW_BITS leading zero bits across bytes
    let full_bytes = (POW_BITS / 8) as usize;
    let rem_bits = POW_BITS % 8;
    for &byte in h.iter().take(full_bytes) {
        if byte != 0 {
            return false;
        }
    }
    if rem_bits > 0 {
        let mask = 0xFF_u8 << (8 - rem_bits);
        if h[full_bytes] & mask != 0 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_is_deterministic() {
        let key = [1u8; 32];
        let id = b"relay-1";
        let t1 = RelayToken::derive(&key, id);
        let t2 = RelayToken::derive(&key, id);
        assert_eq!(t1, t2);
    }

    #[test]
    fn hex_round_trip() {
        let t = RelayToken::random();
        let hex = t.to_hex();
        let t2 = RelayToken::from_hex(&hex).unwrap();
        assert_eq!(t, t2);
    }
}
