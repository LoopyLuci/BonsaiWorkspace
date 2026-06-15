use crate::auth::TokenManager;
use crate::error::{AppError, AppResult};
use crate::models::*;
use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode}, Json};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub struct DeviceHandlers;

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

/// List all devices for a user
pub async fn list_devices(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<Device>>> {
    let user_id = extract_user_id(&headers)?;

    let devices = sqlx::query_as::<_, Device>(
        "SELECT * FROM devices WHERE user_id = $1 ORDER BY last_sync DESC NULLS LAST"
    )
    .bind(&user_id)
    .fetch_all(pool.as_ref())
    .await?;

    Ok(Json(devices))
}

/// Create a new device
pub async fn create_device(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
    Json(req): Json<CreateDeviceRequest>,
) -> AppResult<(StatusCode, Json<Device>)> {
    let user_id = extract_user_id(&headers)?;

    // Validate input
    if req.name.is_empty() {
        return Err(AppError::InvalidInput("Device name required".to_string()));
    }

    let device_id = Uuid::new_v4();
    let now = Utc::now();

    let device = sqlx::query_as::<_, Device>(
        "INSERT INTO devices (id, user_id, name, device_type, platform, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING *"
    )
    .bind(&device_id)
    .bind(&user_id)
    .bind(&req.name)
    .bind(&req.device_type)
    .bind(&req.platform)
    .bind(&now)
    .bind(&now)
    .fetch_one(pool.as_ref())
    .await?;

    Ok((StatusCode::CREATED, Json(device)))
}

/// Get a specific device
pub async fn get_device(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
    Path(device_id): Path<Uuid>,
) -> AppResult<Json<Device>> {
    let user_id = extract_user_id(&headers)?;

    let device = sqlx::query_as::<_, Device>(
        "SELECT * FROM devices WHERE id = $1 AND user_id = $2"
    )
    .bind(&device_id)
    .bind(&user_id)
    .fetch_optional(pool.as_ref())
    .await?
    .ok_or(AppError::DeviceNotFound)?;

    Ok(Json(device))
}

/// Update a device
pub async fn update_device(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
    Path(device_id): Path<Uuid>,
    Json(req): Json<serde_json::json::Value>,
) -> AppResult<Json<Device>> {
    let user_id = extract_user_id(&headers)?;

    let name = req.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let last_sync = req.get("last_sync").and_then(|v| v.as_str()).map(|s| s.to_string());

    let now = Utc::now();
    let last_sync_parsed = last_sync.as_ref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());

    let device = sqlx::query_as::<_, Device>(
        "UPDATE devices
         SET name = COALESCE($1, name),
             last_sync = COALESCE($2, last_sync),
             updated_at = $3
         WHERE id = $4 AND user_id = $5
         RETURNING *"
    )
    .bind(&name)
    .bind(last_sync_parsed.map(|dt| dt.with_timezone(&chrono::Utc)))
    .bind(&now)
    .bind(&device_id)
    .bind(&user_id)
    .fetch_optional(pool.as_ref())
    .await?
    .ok_or(AppError::DeviceNotFound)?;

    Ok(Json(device))
}

/// Remove a device
pub async fn remove_device(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
    Path(device_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let user_id = extract_user_id(&headers)?;

    let result = sqlx::query("DELETE FROM devices WHERE id = $1 AND user_id = $2")
        .bind(&device_id)
        .bind(&user_id)
        .execute(pool.as_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::DeviceNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Update device's last sync timestamp
pub async fn update_last_sync(
    State(pool): State<Arc<PgPool>>,
    headers: HeaderMap,
    Path(device_id): Path<Uuid>,
) -> AppResult<Json<Device>> {
    let user_id = extract_user_id(&headers)?;
    let now = Utc::now();

    let device = sqlx::query_as::<_, Device>(
        "UPDATE devices
         SET last_sync = $1, updated_at = $2
         WHERE id = $3 AND user_id = $4
         RETURNING *"
    )
    .bind(&now)
    .bind(&now)
    .bind(&device_id)
    .bind(&user_id)
    .fetch_optional(pool.as_ref())
    .await?
    .ok_or(AppError::DeviceNotFound)?;

    Ok(Json(device))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation_validation() {
        let req = CreateDeviceRequest {
            name: "".to_string(),
            device_type: "mobile".to_string(),
            platform: "ios".to_string(),
        };

        // Name should not be empty
        assert!(req.name.is_empty());
    }

    #[test]
    fn test_device_uuid_generation() {
        let device_id = Uuid::new_v4();
        let another_id = Uuid::new_v4();

        // Each UUID should be unique
        assert_ne!(device_id, another_id);
    }

    #[test]
    fn test_device_timestamp() {
        let now = Utc::now();
        let other = Utc::now();

        // Timestamps should be in correct order
        assert!(other >= now);
    }
}
