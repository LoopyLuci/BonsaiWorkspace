//! Key derivation: BIP-39 mnemonic → Argon2id → identity seed.
//!
//! The 12-word recovery phrase is the cryptographic root of a Bonsai identity.
//! An optional password/PIN adds a second layer above the phrase.

use argon2::{Argon2, Params, Algorithm, Version};
use bip39::{Mnemonic, Language};
use rand::rngs::OsRng;
use rand::RngCore;
use crate::identity::BonsaiIdentity;
use crate::error::{CryptoError, CryptoResult};

// Argon2id parameters — balanced for desktop security vs. speed.
// Production: memory=65536 KiB (64 MiB), t_cost=3, p_cost=4
// For CI / tests: reduced (see ARGON2_PARAMS_TEST)
const ARGON2_MEMORY_KIB: u32 = 65_536;
const ARGON2_TIME_COST:   u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;

/// Test-speed parameters (1 MiB memory, 1 iteration).
pub const ARGON2_PARAMS_TEST: Params = match Params::new(1_024, 1, 1, Some(32)) {
    Ok(p) => p,
    Err(_) => unreachable!(),
};

/// Generate a new 12-word BIP-39 recovery phrase.
pub fn generate_phrase() -> CryptoResult<String> {
    let mut entropy = [0u8; 16]; // 128 bits → 12 words
    OsRng.fill_bytes(&mut entropy);
    let mnemonic = Mnemonic::from_entropy(&entropy)
        .map_err(|e| CryptoError::InvalidMnemonic(e.to_string()))?;
    Ok(mnemonic.to_string())
}

/// Derive a `BonsaiIdentity` from a BIP-39 phrase (+ optional password).
///
/// `params` can be overridden for tests; pass `None` for production defaults.
pub fn derive_identity_from_phrase(
    phrase: &str,
    password: Option<&str>,
    test_params: Option<Params>,
) -> CryptoResult<BonsaiIdentity> {
    let seed = kdf_phrase_to_seed(phrase, password, test_params)?;
    BonsaiIdentity::from_seed(&seed)
}

/// Derive a 32-byte seed from a BIP-39 phrase + optional password using Argon2id.
pub fn kdf_phrase_to_seed(
    phrase: &str,
    password: Option<&str>,
    test_params: Option<Params>,
) -> CryptoResult<[u8; 32]> {
    // Validate phrase
    let mnemonic = Mnemonic::parse_in(Language::English, phrase)
        .map_err(|e| CryptoError::InvalidMnemonic(e.to_string()))?;

    // Phrase bytes → password for Argon2id
    let phrase_bytes = mnemonic.to_entropy();
    let pwd_layer = password.unwrap_or("");
    let full_password: Vec<u8> = phrase_bytes.iter()
        .chain(pwd_layer.as_bytes())
        .copied()
        .collect();

    // Salt: BLAKE3 of the phrase as a deterministic salt (so derivation is reproducible)
    let salt_hash = blake3::hash(phrase.as_bytes());
    let salt = &salt_hash.as_bytes()[..16];

    let params = test_params.unwrap_or_else(|| {
        Params::new(ARGON2_MEMORY_KIB, ARGON2_TIME_COST, ARGON2_PARALLELISM, Some(32))
            .expect("valid argon2 params")
    });

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut output = [0u8; 32];
    argon2.hash_password_into(&full_password, salt, &mut output)
        .map_err(|e| CryptoError::KdfError(e.to_string()))?;

    Ok(output)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_roundtrip() {
        let phrase = generate_phrase().unwrap();
        let id1 = derive_identity_from_phrase(&phrase, None, Some(ARGON2_PARAMS_TEST)).unwrap();
        let id2 = derive_identity_from_phrase(&phrase, None, Some(ARGON2_PARAMS_TEST)).unwrap();
        assert_eq!(id1.public_key.ed25519_pk, id2.public_key.ed25519_pk,
            "same phrase must yield same identity");
    }

    #[test]
    fn different_phrases_yield_different_identities() {
        let p1 = generate_phrase().unwrap();
        let p2 = generate_phrase().unwrap();
        let id1 = derive_identity_from_phrase(&p1, None, Some(ARGON2_PARAMS_TEST)).unwrap();
        let id2 = derive_identity_from_phrase(&p2, None, Some(ARGON2_PARAMS_TEST)).unwrap();
        assert_ne!(id1.public_key.ed25519_pk, id2.public_key.ed25519_pk);
    }

    #[test]
    fn password_changes_identity() {
        let phrase = generate_phrase().unwrap();
        let id_no_pwd  = derive_identity_from_phrase(&phrase, None,        Some(ARGON2_PARAMS_TEST)).unwrap();
        let id_with_pwd = derive_identity_from_phrase(&phrase, Some("pin"), Some(ARGON2_PARAMS_TEST)).unwrap();
        assert_ne!(id_no_pwd.public_key.ed25519_pk, id_with_pwd.public_key.ed25519_pk);
    }

    #[test]
    fn invalid_phrase_rejected() {
        assert!(derive_identity_from_phrase("not a valid phrase", None, Some(ARGON2_PARAMS_TEST)).is_err());
    }
}
