use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub service_name: String,
    pub status: HealthStatus,
    pub response_time_ms: u64,
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        let result = HealthCheckResult {
            service_name: "api".to_string(),
            status: HealthStatus::Healthy,
            response_time_ms: 50,
            details: "OK".to_string(),
        };
        assert_eq!(result.status, HealthStatus::Healthy);
    }
}
