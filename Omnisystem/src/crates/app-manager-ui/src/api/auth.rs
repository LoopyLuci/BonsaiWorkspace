//! Authentication API endpoints

use serde::{Deserialize, Serialize};
use tauri::command;

/// Login credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub user_id: String,
    pub password: String,
}

/// Login response with token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
}

/// Login command for Tauri
#[command]
pub async fn login(user_id: String, password: String) -> Result<LoginResponse, String> {
    // Validate inputs
    if user_id.is_empty() || password.is_empty() {
        return Err("Username and password required".to_string());
    }

    // Mock authentication for now
    // In production: Call /api/auth/login endpoint
    Ok(LoginResponse {
        access_token: format!("{}.exp.sig", user_id),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        user: UserInfo {
            user_id: user_id.clone(),
            email: format!("{}@example.com", user_id),
            roles: vec!["user".to_string()],
        },
    })
}

/// Logout command for Tauri
#[command]
pub async fn logout() -> Result<(), String> {
    // Clear local token storage
    // In production: Call /api/auth/logout endpoint
    Ok(())
}

/// Verify token validity
#[command]
pub async fn verify_token(token: String) -> Result<bool, String> {
    // Mock token verification
    // In production: Call /api/auth/verify endpoint
    let is_valid = !token.is_empty() && token.contains('.');
    Ok(is_valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_login_success() {
        let result = login("test-user".to_string(), "password123".to_string()).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.token_type, "Bearer");
        assert!(response.expires_in > 0);
        assert_eq!(response.user.user_id, "test-user");
    }

    #[tokio::test]
    async fn test_login_empty_credentials() {
        let result = login(String::new(), "password".to_string()).await;
        assert!(result.is_err());

        let result = login("user".to_string(), String::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_logout() {
        let result = logout().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_token_valid() {
        let result = verify_token("user.123456.sig".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_verify_token_invalid() {
        let result = verify_token("invalid-token".to_string()).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
