use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::metrics::DashboardMetrics;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub tenants: Vec<crate::metrics::TenantMetrics>,
    pub providers: Vec<crate::metrics::ProviderMetrics>,
    pub timestamp: u64,
}

pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

pub async fn metrics_handler(metrics: Arc<DashboardMetrics>) -> Json<MetricsResponse> {
    let (tenants, providers) = metrics.get_all_metrics().await.unwrap_or_default();

    Json(MetricsResponse {
        tenants,
        providers,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_handler() {
        let response = health_handler().await;
        assert_eq!(response.status, "healthy");
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            timestamp: 1000,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
    }
}
