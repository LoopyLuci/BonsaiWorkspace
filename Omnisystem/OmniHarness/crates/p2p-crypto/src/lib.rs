//! p2p-crypto — Post-quantum hybrid cryptography for BonsaiWorkspace.
//!
//! Implements the TransferDaemon cryptographic model:
//! - Hybrid key exchange: X25519 + ML-KEM-768 (FIPS 203 / Kyber)
//! - Identity keys: Ed25519 (classical) — ML-DSA-87 via feature when available
//! - Authenticated encryption: AES-256-GCM with BLAKE3 integrity
//! - Key derivation: Argon2id from BIP-39 12-word recovery phrase
//! - Forward secrecy: ephemeral session keys per transfer

pub mod cipher;
pub mod error;
pub mod identity;
pub mod kdf;
pub mod session;

pub use cipher::{decrypt_chunk, encrypt_chunk, ChunkCiphertext};
pub use error::{CryptoError, CryptoResult};
pub use identity::{WorkspaceIdentity, IdentityPublicKey};
pub use kdf::{derive_identity_from_phrase, generate_phrase};
pub use session::{HybridHandshake, InitiatorHello, ResponderHello, SessionKey};
