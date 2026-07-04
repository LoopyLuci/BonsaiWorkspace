#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub struct HealthMonitor;

impl HealthMonitor {
    pub async fn check() -> anyhow::Result<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let status = HealthMonitor::check().await.unwrap();
        matches!(status, HealthStatus::Healthy);
    }
}
