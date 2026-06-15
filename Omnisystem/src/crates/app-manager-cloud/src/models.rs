use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============ User Models ============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip)]
    pub password_hash: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserProfile,
    pub token: TokenResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

// ============ Device Models ============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub device_type: String,
    pub platform: String,
    pub device_token: Option<String>,
    pub last_sync: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceRequest {
    pub name: String,
    pub device_type: String,
    pub platform: String,
}

// ============ Favorite Models ============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Favorite {
    pub id: Uuid,
    pub user_id: Uuid,
    pub app_id: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFavoriteRequest {
    pub app_id: String,
}

// ============ Settings Models ============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSettings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub theme: String,
    pub language: String,
    pub notifications_enabled: bool,
    pub auto_update: bool,
    pub sync_frequency: String,
    pub download_quality: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub notifications_enabled: Option<bool>,
    pub auto_update: Option<bool>,
    pub sync_frequency: Option<String>,
    pub download_quality: Option<String>,
}

// ============ Installation Models ============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Installation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub app_id: String,
    pub version: Option<String>,
    pub install_date: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub size_mb: Option<i32>,
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub version_num: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInstallationRequest {
    pub app_id: String,
    pub version: String,
    pub size_mb: i32,
}

// ============ Sync Models ============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub change_data: Option<serde_json::Value>,
    pub version: Option<i32>,
    pub synced_by_device_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncPushRequest {
    pub changes: Vec<ChangeLog>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeLog {
    pub id: String,
    #[serde(rename = "type")]
    pub change_type: String,
    pub resource_type: String,
    pub resource_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub synced: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncPushResponse {
    pub synced: Vec<ChangeLog>,
    pub conflicts: Vec<SyncConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncConflict {
    pub id: Uuid,
    pub user_id: Uuid,
    pub resource_type: String,
    pub resource_id: String,
    pub device_id_1: Option<Uuid>,
    pub device_id_2: Option<Uuid>,
    pub local_version: Option<serde_json::Value>,
    pub remote_version: Option<serde_json::Value>,
    pub resolution: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    pub resolution: String,
}

// ============ Review Models ============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Review {
    pub id: Uuid,
    pub user_id: Uuid,
    pub app_id: String,
    pub rating: i32,
    pub title: Option<String>,
    pub content: Option<String>,
    pub helpful_count: i32,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub app_id: String,
    pub rating: i32,
    pub title: Option<String>,
    pub content: Option<String>,
}

// ============ Error Models ============

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

// ============ Audit Models ============

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub result: String,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

// ============ JWT Claims ============

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub email: String,
    pub exp: i64,
    pub iat: i64,
    pub nbf: i64,
    pub roles: Vec<String>,
}
