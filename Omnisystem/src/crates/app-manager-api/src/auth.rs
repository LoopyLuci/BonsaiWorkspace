//! JWT authentication and authorization

use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};
use std::fmt;

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // Subject (user ID)
    pub user_id: String,       // User identifier
    pub email: String,         // User email
    pub roles: Vec<String>,    // User roles
    pub exp: i64,              // Expiration time
    pub iat: i64,              // Issued at
    pub nbf: i64,              // Not before
}

impl Claims {
    /// Create new claims with 1-hour expiration
    pub fn new(user_id: String, email: String, roles: Vec<String>) -> Self {
        let now = Utc::now();
        let exp = (now + Duration::hours(1)).timestamp();

        Self {
            sub: user_id.clone(),
            user_id,
            email,
            roles,
            exp,
            iat: now.timestamp(),
            nbf: now.timestamp(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Get time until expiration (in seconds)
    pub fn expires_in(&self) -> i64 {
        self.exp - Utc::now().timestamp()
    }

    /// Check if user has required role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

/// Token generation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Authentication request
#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub user_id: String,
    pub password: String,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

/// User information in response
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
}

/// Authentication errors
#[derive(Debug, Clone, Serialize)]
pub enum AuthError {
    InvalidCredentials,
    TokenExpired,
    InvalidToken,
    MissingToken,
    InsufficientPermissions,
    InvalidRole,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "Invalid credentials"),
            Self::TokenExpired => write!(f, "Token has expired"),
            Self::InvalidToken => write!(f, "Invalid or malformed token"),
            Self::MissingToken => write!(f, "Missing authorization token"),
            Self::InsufficientPermissions => write!(f, "Insufficient permissions for this operation"),
            Self::InvalidRole => write!(f, "Invalid role specification"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Mock JWT encoder/decoder for Phase 2
/// In production (Phase 3+), use jsonwebtoken crate
pub struct TokenManager;

impl TokenManager {
    /// Generate a JWT token (mock implementation)
    /// In production: Use jsonwebtoken::encode() with HS256
    pub fn generate_token(claims: &Claims) -> Result<String, AuthError> {
        // Mock token format: user_id.exp.signature
        // Real implementation would use RS256 or HS256
        let token = format!(
            "{}.{}.{}",
            claims.user_id,
            claims.exp,
            "signature_placeholder"
        );
        Ok(token)
    }

    /// Verify and decode JWT token (mock implementation)
    /// In production: Use jsonwebtoken::decode() with secret
    pub fn verify_token(token: &str) -> Result<Claims, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken);
        }

        let user_id = parts[0].to_string();
        let exp_str = parts[1];

        let exp: i64 = exp_str
            .parse()
            .map_err(|_| AuthError::InvalidToken)?;

        if Utc::now().timestamp() > exp {
            return Err(AuthError::TokenExpired);
        }

        // Return mock claims
        Ok(Claims {
            sub: user_id.clone(),
            user_id,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
            exp,
            iat: Utc::now().timestamp(),
            nbf: Utc::now().timestamp(),
        })
    }

    /// Extract token from Authorization header
    pub fn extract_token(auth_header: &str) -> Result<String, AuthError> {
        let parts: Vec<&str> = auth_header.split_whitespace().collect();

        if parts.len() != 2 {
            return Err(AuthError::MissingToken);
        }

        if parts[0].to_lowercase() != "bearer" {
            return Err(AuthError::InvalidToken);
        }

        Ok(parts[1].to_string())
    }
}

/// Role-based access control
pub struct RoleChecker;

impl RoleChecker {
    pub const ROLE_ADMIN: &'static str = "admin";
    pub const ROLE_USER: &'static str = "user";
    pub const ROLE_PUBLISHER: &'static str = "publisher";
    pub const ROLE_INSTALLER: &'static str = "installer";

    /// Check if user can perform admin operations
    pub fn is_admin(claims: &Claims) -> bool {
        claims.has_role(Self::ROLE_ADMIN)
    }

    /// Check if user can perform user operations
    pub fn is_user(claims: &Claims) -> bool {
        claims.has_role(Self::ROLE_USER) || claims.has_role(Self::ROLE_ADMIN)
    }

    /// Check if user can publish apps
    pub fn is_publisher(claims: &Claims) -> bool {
        claims.has_role(Self::ROLE_PUBLISHER) || claims.has_role(Self::ROLE_ADMIN)
    }

    /// Check if user can install apps
    pub fn is_installer(claims: &Claims) -> bool {
        claims.has_role(Self::ROLE_INSTALLER) || claims.has_role(Self::ROLE_ADMIN)
    }

    /// Validate operation requires specific role
    pub fn require_role(claims: &Claims, required_role: &str) -> Result<(), AuthError> {
        if claims.has_role(required_role) || claims.has_role(Self::ROLE_ADMIN) {
            Ok(())
        } else {
            Err(AuthError::InsufficientPermissions)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new(
            "user-123".to_string(),
            "user@example.com".to_string(),
            vec!["user".to_string()],
        );

        assert_eq!(claims.user_id, "user-123");
        assert_eq!(claims.email, "user@example.com");
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_claims_has_role() {
        let claims = Claims::new(
            "user-123".to_string(),
            "user@example.com".to_string(),
            vec!["user".to_string(), "publisher".to_string()],
        );

        assert!(claims.has_role("user"));
        assert!(claims.has_role("publisher"));
        assert!(!claims.has_role("admin"));
    }

    #[test]
    fn test_role_checker_permissions() {
        let user = Claims::new(
            "user-1".to_string(),
            "user@example.com".to_string(),
            vec!["user".to_string()],
        );

        let admin = Claims::new(
            "admin-1".to_string(),
            "admin@example.com".to_string(),
            vec!["admin".to_string()],
        );

        assert!(RoleChecker::is_user(&user));
        assert!(!RoleChecker::is_admin(&user));
        assert!(RoleChecker::is_admin(&admin));
    }

    #[test]
    fn test_token_manager_generate() {
        let claims = Claims::new(
            "user-123".to_string(),
            "user@example.com".to_string(),
            vec!["user".to_string()],
        );

        let result = TokenManager::generate_token(&claims);
        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(token.contains("user-123"));
    }

    #[test]
    fn test_token_extraction() {
        let result = TokenManager::extract_token("Bearer valid.token.here");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid.token.here");

        let bad_result = TokenManager::extract_token("Basic user:pass");
        assert!(bad_result.is_err());
    }

    #[test]
    fn test_role_requirement() {
        let claims = Claims::new(
            "user-1".to_string(),
            "user@example.com".to_string(),
            vec!["user".to_string()],
        );

        assert!(RoleChecker::require_role(&claims, "user").is_ok());
        assert!(RoleChecker::require_role(&claims, "admin").is_err());
    }

    #[test]
    fn test_auth_error_display() {
        assert_eq!(
            AuthError::InvalidCredentials.to_string(),
            "Invalid credentials"
        );
        assert_eq!(AuthError::TokenExpired.to_string(), "Token has expired");
    }
}
