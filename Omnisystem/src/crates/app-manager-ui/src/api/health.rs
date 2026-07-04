//! Health check API endpoint

use serde::{Deserialize, Serialize};
use tauri::command;

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub api_available: bool,
    pub database_available: bool,
    pub timestamp: String,
}

/// Check API health
#[command]
pub async fn check_api_health() -> Result<HealthStatus, String> {
    // Mock implementation
    // In production: Call GET /api/health
    Ok(HealthStatus {
        status: "healthy".to_string(),
        api_available: true,
        database_available: true,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_api_health() {
        let result = check_api_health().await;
        assert!(result.is_ok());

        let health = result.unwrap();
        assert_eq!(health.status, "healthy");
        assert!(health.api_available);
        assert!(health.database_available);
    }

    #[test]
    fn test_health_status_timestamp() {
        let health = HealthStatus {
            status: "healthy".to_string(),
            api_available: true,
            database_available: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        assert!(!health.timestamp.is_empty());
        assert!(health.timestamp.contains('T'));
    }
}
