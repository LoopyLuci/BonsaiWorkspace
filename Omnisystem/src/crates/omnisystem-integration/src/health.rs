use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub modules_healthy: usize,
    pub modules_total: usize,
}

pub struct HealthCheck;

impl HealthCheck {
    pub fn check(healthy: usize, total: usize) -> HealthStatus {
        HealthStatus {
            status: if healthy == total {
                "healthy".to_string()
            } else {
                "degraded".to_string()
            },
            modules_healthy: healthy,
            modules_total: total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_health_check() {
        let health = HealthCheck::check(5, 5);
        assert_eq!(health.status, "healthy");
    }
}
