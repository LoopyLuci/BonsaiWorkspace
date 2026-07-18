// Authentication utilities for PATHFINDER User Service

use anyhow::{anyhow, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};
use std::sync::OnceLock;

const MIN_JWT_SECRET_LEN: usize = 32;
static JWT_SECRET: OnceLock<String> = OnceLock::new();

/// Load the JWT signing secret from the `JWT_SECRET` environment variable.
/// Fails closed: no hardcoded fallback, and a too-short secret is rejected.
fn jwt_secret() -> Result<&'static str> {
    if let Some(secret) = JWT_SECRET.get() {
        return Ok(secret.as_str());
    }
    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| anyhow!("JWT_SECRET environment variable is not set"))?;
    if secret.len() < MIN_JWT_SECRET_LEN {
        return Err(anyhow!(
            "JWT_SECRET must be at least {MIN_JWT_SECRET_LEN} characters"
        ));
    }
    Ok(JWT_SECRET.get_or_init(|| secret).as_str())
}

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // Subject (user ID)
    pub exp: i64,     // Expiration time
    pub iat: i64,     // Issued at
    pub iss: String,  // Issuer
}

/// Hash password with bcrypt
pub fn hash_password(password: &str) -> Result<String> {
    let hashed = hash(password, DEFAULT_COST)?;
    Ok(hashed)
}

/// Verify password against bcrypt hash
pub fn verify_password(password: &str, hash: &str) -> Result<()> {
    let valid = verify(password, hash)?;
    if valid {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Invalid password"))
    }
}

/// Generate JWT token
pub fn generate_jwt(user_id: &str) -> Result<String> {
    let secret = jwt_secret()?;
    let now = Utc::now();
    let expires_at = now + Duration::days(1);

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expires_at.timestamp(),
        iat: now.timestamp(),
        iss: "pathfinder".to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )?;

    Ok(token)
}

/// Validate JWT token
pub fn validate_jwt(token: &str) -> Result<Claims> {
    let secret = jwt_secret()?;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )?;

    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All tests in this module share one process, and `jwt_secret()`
    /// caches the secret in a `OnceLock` the first time it's read -- so
    /// every test that needs a secret sets the *same* value before
    /// touching JWT functions. Whichever test runs first actually sets
    /// it; the rest are no-ops against an already-initialized secret.
    fn ensure_jwt_secret() {
        std::env::set_var("JWT_SECRET", "test-secret-at-least-32-characters-long");
    }

    #[test]
    fn test_hash_and_verify_password_roundtrip() {
        let hashed = hash_password("correct horse battery staple").unwrap();
        assert!(hashed.starts_with("$2"));
        assert!(verify_password("correct horse battery staple", &hashed).is_ok());
    }

    #[test]
    fn test_verify_password_rejects_wrong_password() {
        let hashed = hash_password("right-password").unwrap();
        assert!(verify_password("wrong-password", &hashed).is_err());
    }

    #[test]
    fn test_generate_and_validate_jwt_roundtrip() {
        ensure_jwt_secret();
        let token = generate_jwt("user-42").unwrap();
        let claims = validate_jwt(&token).unwrap();
        assert_eq!(claims.sub, "user-42");
        assert_eq!(claims.iss, "pathfinder");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_validate_jwt_rejects_garbage_token() {
        ensure_jwt_secret();
        assert!(validate_jwt("not-a-real-token").is_err());
    }

    #[test]
    fn test_validate_jwt_rejects_token_signed_with_different_secret() {
        ensure_jwt_secret();
        // Sign a token with a different secret than the process-wide one
        // used by generate_jwt/validate_jwt, and confirm it's rejected.
        let claims = Claims {
            sub: "user-99".to_string(),
            exp: (Utc::now() + Duration::days(1)).timestamp(),
            iat: Utc::now().timestamp(),
            iss: "pathfinder".to_string(),
        };
        let bogus_token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"a-completely-different-32-char-secret"),
        )
        .unwrap();

        assert!(validate_jwt(&bogus_token).is_err());
    }
}
