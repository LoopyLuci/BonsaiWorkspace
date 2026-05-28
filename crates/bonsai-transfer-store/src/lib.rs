//! Encrypted persistent store for Bonsai transfer state and identity data.
//!
//! Binary format: `MAGIC(8) | SALT(16) | NONCE(12) | CIPHERTEXT(n) | BLAKE3_TAG(32)`
//!
//! Key derivation: Argon2id(passphrase, SALT) → 32-byte AES-256-GCM key.
//! Integrity: BLAKE3 over the entire file header + ciphertext.

pub mod store;
pub use store::{EncryptedStore, StoreError, StoreResult};
