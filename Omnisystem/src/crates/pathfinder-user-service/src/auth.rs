// Authentication utilities for PATHFINDER User Service

use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};

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
    let secret = "your-secret-key-for-jwt"; // In production, use environment variable
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
    let secret = "your-secret-key-for-jwt";

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )?;

    Ok(data.claims)
}
