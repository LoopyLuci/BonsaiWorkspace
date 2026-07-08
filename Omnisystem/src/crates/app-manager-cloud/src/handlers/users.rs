use crate::auth::TokenManager;
use crate::error::{AppError, AppResult};
use crate::models::*;
use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserHandlers;

/// Extract user ID from authorization header
fn extract_user_id(headers: &HeaderMap) -> AppResult<Uuid> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let claims = TokenManager::verify_token(token)?;
    Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::InvalidToken("Invalid user ID in token".to_string()))
}

/// Get current user profile
pub async fn get_profile(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
) -> AppResult<Json<UserProfile>> {
    let user_id = extract_user_id(&headers)?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_optional(pool.as_ref())
        .await?
        .ok_or(AppError::UserNotFound)?;

    let profile = UserProfile {
        id: user.id,
        email: user.email,
        name: user.name,
        avatar_url: user.avatar_url,
        created_at: user.created_at,
    };

    Ok(Json(profile))
}

/// Update user profile
pub async fn update_profile(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
    Json(req): Json<serde_json::json::Value>,
) -> AppResult<Json<UserProfile>> {
    let user_id = extract_user_id(&headers)?;

    // Extract update fields
    let name = req.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let avatar_url = req.get("avatar_url").and_then(|v| v.as_str()).map(|s| s.to_string());

    let now = Utc::now();

    let user = sqlx::query_as::<_, User>(
        "UPDATE users
         SET name = COALESCE($1, name),
             avatar_url = COALESCE($2, avatar_url),
             updated_at = $3
         WHERE id = $4
         RETURNING *"
    )
    .bind(&name)
    .bind(&avatar_url)
    .bind(&now)
    .bind(&user_id)
    .fetch_optional(pool.as_ref())
    .await?
    .ok_or(AppError::UserNotFound)?;

    let profile = UserProfile {
        id: user.id,
        email: user.email,
        name: user.name,
        avatar_url: user.avatar_url,
        created_at: user.created_at,
    };

    Ok(Json(profile))
}

/// Delete user account (and all associated data)
pub async fn delete_account(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let user_id = extract_user_id(&headers)?;

    // Start transaction
    let mut tx = pool.begin().await?;

    // Delete user and cascade to all related tables
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(&user_id)
        .execute(&mut *tx)
        .await?;

    // Commit transaction
    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

/// List user's devices
pub async fn list_devices(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<Device>>> {
    let user_id = extract_user_id(&headers)?;

    let devices = sqlx::query_as::<_, Device>(
        "SELECT * FROM devices WHERE user_id = $1 ORDER BY last_sync DESC"
    )
    .bind(&user_id)
    .fetch_all(pool.as_ref())
    .await?;

    Ok(Json(devices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_verification() {
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

        let token = TokenManager::generate_token(&user).unwrap();
        let claims = TokenManager::verify_token(&token).unwrap();

        assert_eq!(claims.sub, user.id.to_string());
        assert_eq!(claims.email, user.email);
    }

    #[test]
    fn test_expired_token_rejection() {
        // Token generated with past expiration
        let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyLCJleHAiOjE1MTYyMzkwMjJ9.TJVA95OrM7E2cBab30RMHrHDcEfxjoYZgeFONFh7HgQ";
        let result = TokenManager::verify_token(token);
        assert!(result.is_err());
    }
}
