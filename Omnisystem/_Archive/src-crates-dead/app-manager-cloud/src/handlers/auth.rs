use crate::auth::{PasswordManager, TokenManager};
use crate::error::{AppError, AppResult};
use crate::models::*;
use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub struct AuthHandlers;

/// Register a new user
pub async fn register(
    State(pool): State<Arc<PgPool>>,
    Json(req): Json<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    // Validate input
    if req.email.is_empty() || req.password.is_empty() {
        return Err(AppError::InvalidInput("Email and password required".to_string()));
    }

    // Check if email already exists
    let existing = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE email = $1")
        .bind(&req.email)
        .fetch_one(pool.as_ref())
        .await?;

    if existing > 0 {
        return Err(AppError::EmailAlreadyExists);
    }

    // Hash password
    let password_hash = PasswordManager::hash_password(&req.password)?;

    // Create user
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, password_hash, name, created_at, updated_at, is_active)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING *"
    )
    .bind(&user_id)
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.name)
    .bind(&now)
    .bind(&now)
    .bind(true)
    .fetch_one(pool.as_ref())
    .await?;

    // Generate tokens
    let access_token = TokenManager::generate_token(&user)?;
    let refresh_token = TokenManager::generate_refresh_token(user.id)?;

    // Create default settings
    let settings_id = Uuid::new_v4();
    let _ = sqlx::query(
        "INSERT INTO user_settings (id, user_id, theme, language, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(&settings_id)
    .bind(&user.id)
    .bind("auto")
    .bind("en")
    .bind(&now)
    .bind(&now)
    .execute(pool.as_ref())
    .await;

    let response = AuthResponse {
        user: UserProfile {
            id: user.id,
            email: user.email,
            name: user.name,
            avatar_url: user.avatar_url,
            created_at: user.created_at,
        },
        token: TokenResponse {
            access_token,
            refresh_token,
            expires_in: 86400, // 24 hours
            token_type: "Bearer".to_string(),
        },
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// Login a user
pub async fn login(
    State(pool): State<Arc<PgPool>>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    // Find user by email
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND is_active = true")
        .bind(&req.email)
        .fetch_optional(pool.as_ref())
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    // Verify password
    let password_valid = PasswordManager::verify_password(&req.password, &user.password_hash)?;
    if !password_valid {
        return Err(AppError::InvalidCredentials);
    }

    // Update last login
    let _ = sqlx::query("UPDATE users SET last_login = $1 WHERE id = $2")
        .bind(Utc::now())
        .bind(&user.id)
        .execute(pool.as_ref())
        .await;

    // Generate tokens
    let access_token = TokenManager::generate_token(&user)?;
    let refresh_token = TokenManager::generate_refresh_token(user.id)?;

    let response = AuthResponse {
        user: UserProfile {
            id: user.id,
            email: user.email,
            name: user.name,
            avatar_url: user.avatar_url,
            created_at: user.created_at,
        },
        token: TokenResponse {
            access_token,
            refresh_token,
            expires_in: 86400,
            token_type: "Bearer".to_string(),
        },
    };

    Ok(Json(response))
}

/// Refresh access token
pub async fn refresh_token(
    State(pool): State<Arc<PgPool>>,
    Json(req): Json<serde_json::json::Value>,
) -> AppResult<Json<TokenResponse>> {
    // Extract refresh token from request
    let refresh_token = req
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or(AppError::InvalidInput("refresh_token required".to_string()))?;

    // Verify refresh token and get user_id
    let user_id = TokenManager::verify_refresh_token(refresh_token)?;

    // Fetch user
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(pool.as_ref())
        .await?
        .ok_or(AppError::UserNotFound)?;

    // Generate new access token
    let access_token = TokenManager::generate_token(&user)?;
    let new_refresh_token = TokenManager::generate_refresh_token(user.id)?;

    let response = TokenResponse {
        access_token,
        refresh_token: new_refresh_token,
        expires_in: 86400,
        token_type: "Bearer".to_string(),
    };

    Ok(Json(response))
}

/// Logout a user (invalidate tokens server-side - future enhancement)
pub async fn logout(
    State(pool): State<Arc<PgPool>>,
) -> AppResult<Json<serde_json::json::Value>> {
    // In a production system, you would:
    // 1. Extract user from JWT
    // 2. Add token to a blacklist/invalidation table
    // 3. Return success

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_manager() {
        let password = "TestPassword123!";
        let hash = PasswordManager::hash_password(password);
        assert!(hash.is_ok());

        let verify = PasswordManager::verify_password(password, &hash.unwrap());
        assert!(verify.is_ok());
        assert!(verify.unwrap());
    }

    #[test]
    fn test_token_generation() {
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

        let token = TokenManager::generate_token(&user);
        assert!(token.is_ok());

        let verify = TokenManager::verify_token(&token.unwrap());
        assert!(verify.is_ok());
    }

    #[test]
    fn test_refresh_token_generation() {
        let user_id = Uuid::new_v4();
        let token = TokenManager::generate_refresh_token(user_id);
        assert!(token.is_ok());

        let verify = TokenManager::verify_refresh_token(&token.unwrap());
        assert!(verify.is_ok());
        assert_eq!(verify.unwrap(), user_id);
    }
}
