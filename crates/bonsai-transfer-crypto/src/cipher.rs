//! AES-256-GCM chunk encryption fused with BLAKE3 integrity.
//!
//! Each chunk gets a unique 96-bit nonce derived from the chunk's GSN and a
//! per-session nonce seed, ensuring nonce uniqueness without a counter per-key.

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use serde::{Deserialize, Serialize};
use crate::session::SessionKey;
use crate::error::{CryptoError, CryptoResult};

// ── Chunk ciphertext ──────────────────────────────────────────────────────────

/// An encrypted + authenticated chunk ready for transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkCiphertext {
    /// 96-bit AES-GCM nonce.
    pub nonce: [u8; 12],
    /// Ciphertext + 16-byte GCM authentication tag.
    pub ciphertext: Vec<u8>,
    /// BLAKE3 hash of the *plaintext* for deduplication / integrity verification.
    pub plaintext_hash: [u8; 32],
    /// Global Sequence Number this chunk belongs to.
    pub gsn: u64,
}

// ── Encrypt ───────────────────────────────────────────────────────────────────

/// Encrypt `plaintext` as chunk `gsn` using `session_key`.
pub fn encrypt_chunk(session_key: &SessionKey, gsn: u64, plaintext: &[u8]) -> CryptoResult<ChunkCiphertext> {
    let key = Key::<Aes256Gcm>::from_slice(session_key.as_bytes());
    let cipher = Aes256Gcm::new(key);

    // Nonce: 8 bytes from GSN (big-endian) + 4 bytes random
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes[..8].copy_from_slice(&gsn.to_be_bytes());
    let rand_suffix: [u8; 4] = rand::random();
    nonce_bytes[8..].copy_from_slice(&rand_suffix);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::EncryptionError("AES-GCM encrypt failed".into()))?;

    let plaintext_hash = *blake3::hash(plaintext).as_bytes();

    Ok(ChunkCiphertext {
        nonce: nonce_bytes,
        ciphertext,
        plaintext_hash,
        gsn,
    })
}

// ── Decrypt ───────────────────────────────────────────────────────────────────

/// Decrypt and authenticate `chunk` using `session_key`.
pub fn decrypt_chunk(session_key: &SessionKey, chunk: &ChunkCiphertext) -> CryptoResult<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(session_key.as_bytes());
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&chunk.nonce);

    let plaintext = cipher.decrypt(nonce, chunk.ciphertext.as_slice())
        .map_err(|_| CryptoError::DecryptionFailed)?;

    // Verify BLAKE3 hash matches
    let hash = *blake3::hash(&plaintext).as_bytes();
    if hash != chunk.plaintext_hash {
        return Err(CryptoError::DecryptionFailed);
    }

    Ok(plaintext)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionKey;

    fn test_key() -> SessionKey {
        SessionKey([42u8; 32])
    }

    #[test]
    fn roundtrip() {
        let key = test_key();
        let plain = b"Hello, Bonsai transfer engine!";
        let ct = encrypt_chunk(&key, 0, plain).unwrap();
        let recovered = decrypt_chunk(&key, &ct).unwrap();
        assert_eq!(recovered, plain);
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = test_key();
        let mut ct = encrypt_chunk(&key, 1, b"secret data").unwrap();
        ct.ciphertext[0] ^= 0xFF;
        assert!(decrypt_chunk(&key, &ct).is_err());
    }

    #[test]
    fn gsn_in_nonce() {
        let key = test_key();
        let ct0 = encrypt_chunk(&key, 0, b"chunk 0").unwrap();
        let ct1 = encrypt_chunk(&key, 1, b"chunk 1").unwrap();
        // The first 8 bytes of each nonce encode the GSN
        assert_ne!(ct0.nonce[..8], ct1.nonce[..8]);
    }
}
