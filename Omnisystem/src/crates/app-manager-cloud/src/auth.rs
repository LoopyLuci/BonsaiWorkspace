use crate::error::{AppError, AppResult};
use crate::models::{Claims, User};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

const JWT_SECRET: &str = "your-secret-key-min-32-chars-long!"; // Use env var in production
const TOKEN_EXPIRATION_HOURS: i64 = 24;
const REFRESH_TOKEN_EXPIRATION_DAYS: i64 = 30;

pub struct TokenManager;

impl TokenManager {
    /// Generate a new access token
    pub fn generate_token(user: &User) -> AppResult<String> {
        let now = Utc::now();
        let exp = (now + Duration::hours(TOKEN_EXPIRATION_HOURS)).timestamp();
        let iat = now.timestamp();
        let nbf = iat;

        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            exp,
            iat,
            nbf,
            roles: vec!["user".to_string()],
        };

        let key = EncodingKey::from_secret(JWT_SECRET.as_ref());
        encode(&Header::default(), &claims, &key)
            .map_err(|_| AppError::InternalServerError)
    }

    /// Generate a refresh token
    pub fn generate_refresh_token(user_id: Uuid) -> AppResult<String> {
        let now = Utc::now();
        let exp = (now + Duration::days(REFRESH_TOKEN_EXPIRATION_DAYS)).timestamp();
        let iat = now.timestamp();
        let nbf = iat;

        let claims = Claims {
            sub: user_id.to_string(),
            email: String::new(),
            exp,
            iat,
            nbf,
            roles: vec!["refresh".to_string()],
        };

        let key = EncodingKey::from_secret(JWT_SECRET.as_ref());
        encode(&Header::default(), &claims, &key)
            .map_err(|_| AppError::InternalServerError)
    }

    /// Verify and decode a token
    pub fn verify_token(token: &str) -> AppResult<Claims> {
        let key = DecodingKey::from_secret(JWT_SECRET.as_ref());
        let validation = Validation::default();

        decode::<Claims>(token, &key, &validation)
            .map(|data| data.claims)
            .map_err(|err| {
                if err.kind() == &jsonwebtoken::error::ErrorKind::ExpiredSignature {
                    AppError::TokenExpired
                } else {
                    AppError::InvalidToken(err.to_string())
                }
            })
    }

    /// Verify refresh token
    pub fn verify_refresh_token(token: &str) -> AppResult<Uuid> {
        let claims = Self::verify_token(token)?;

        if !claims.roles.contains(&"refresh".to_string()) {
            return Err(AppError::InvalidToken("Not a refresh token".to_string()));
        }

        Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::InvalidToken("Invalid user ID".to_string()))
    }
}

pub struct PasswordManager;

impl PasswordManager {
    /// Hash a password using bcrypt
    pub fn hash_password(password: &str) -> AppResult<String> {
        bcrypt::hash(password, 12)
            .map_err(|_| AppError::InternalServerError)
    }

    /// Verify a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
        bcrypt::verify(password, hash)
            .map_err(|_| AppError::InternalServerError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            name: Some("Test User".to_string()),
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            is_active: true,
        };

        let token = TokenManager::generate_token(&user);
        assert!(token.is_ok());
    }

    #[test]
    fn test_verify_token() {
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            name: None,
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            is_active: true,
        };

        let token = TokenManager::generate_token(&user).unwrap();
        let claims = TokenManager::verify_token(&token);
        assert!(claims.is_ok());
    }

    #[test]
    fn test_invalid_token() {
        let result = TokenManager::verify_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_password() {
        let password = "MySecurePassword123!";
        let hash = PasswordManager::hash_password(password);
        assert!(hash.is_ok());
    }

    #[test]
    fn test_verify_password() {
        let password = "MySecurePassword123!";
        let hash = PasswordManager::hash_password(password).unwrap();
        let result = PasswordManager::verify_password(password, &hash);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_wrong_password() {
        let password = "MySecurePassword123!";
        let hash = PasswordManager::hash_password(password).unwrap();
        let result = PasswordManager::verify_password("WrongPassword", &hash);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
