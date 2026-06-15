//! API module for backend communication

pub mod auth;
pub mod apps;
pub mod marketplace;
pub mod settings;
pub mod health;
pub mod statistics;
pub mod favorites;
pub mod telemetry;

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// API client for communicating with backend
pub struct ApiClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl ApiClient {
    /// Create new API client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            token: None,
        }
    }

    /// Default client pointing to localhost
    pub fn default() -> Self {
        Self::new("http://localhost:8080")
    }

    /// Set authentication token
    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    /// Clear authentication token
    pub fn clear_token(&mut self) {
        self.token = None;
    }

    /// Get authorization header
    fn get_auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    /// Build full URL
    fn build_url(&self, endpoint: &str) -> String {
        format!("{}{}", self.base_url, endpoint)
    }
}

/// Generic API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn is_ok(&self) -> bool {
        self.success && self.data.is_some()
    }

    pub fn is_err(&self) -> bool {
        !self.success || self.error.is_some()
    }
}

/// API error type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiError {
    RequestFailed(String),
    Unauthorized,
    NotFound,
    BadRequest(String),
    ServerError(String),
    NetworkError(String),
    InvalidResponse,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestFailed(msg) => write!(f, "Request failed: {}", msg),
            Self::Unauthorized => write!(f, "Unauthorized"),
            Self::NotFound => write!(f, "Not found"),
            Self::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            Self::ServerError(msg) => write!(f, "Server error: {}", msg),
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::InvalidResponse => write!(f, "Invalid response from server"),
        }
    }
}

impl std::error::Error for ApiError {}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new("http://localhost:8080");
        assert_eq!(client.base_url, "http://localhost:8080");
        assert!(client.token.is_none());
    }

    #[test]
    fn test_api_client_token_management() {
        let mut client = ApiClient::new("http://localhost:8080");
        assert!(client.token.is_none());

        client.set_token("test-token".to_string());
        assert!(client.token.is_some());
        assert_eq!(client.get_auth_header(), Some("Bearer test-token".to_string()));

        client.clear_token();
        assert!(client.token.is_none());
    }

    #[test]
    fn test_api_client_url_building() {
        let client = ApiClient::new("http://localhost:8080");
        let url = client.build_url("/api/apps");
        assert_eq!(url, "http://localhost:8080/api/apps");
    }

    #[test]
    fn test_api_response_success() {
        let response: ApiResponse<String> = ApiResponse {
            success: true,
            data: Some("test".to_string()),
            error: None,
        };

        assert!(response.is_ok());
        assert!(!response.is_err());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<String> = ApiResponse {
            success: false,
            data: None,
            error: Some("error".to_string()),
        };

        assert!(!response.is_ok());
        assert!(response.is_err());
    }
}
