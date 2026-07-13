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
