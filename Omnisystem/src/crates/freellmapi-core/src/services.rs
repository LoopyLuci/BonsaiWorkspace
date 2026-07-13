use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait OmnisystemService: Send + Sync {
    fn service_id(&self) -> &str;
    fn service_name(&self) -> &str;
    fn version(&self) -> &str;
    async fn initialize(&self) -> anyhow::Result<()>;
    async fn health_check(&self) -> anyhow::Result<bool>;
    async fn shutdown(&self) -> anyhow::Result<()>;
}

#[async_trait]
pub trait StorageRepository: Send + Sync {
    async fn create_tenant(&self, tenant: &crate::Tenant) -> anyhow::Result<()>;
    async fn get_tenant(&self, id: &str) -> anyhow::Result<crate::Tenant>;
    async fn create_api_key(&self, key: &crate::ApiKey) -> anyhow::Result<()>;
    async fn get_api_key_by_hash(&self, hash: &str) -> anyhow::Result<crate::ApiKey>;
    async fn log_request(&self, log: &crate::RequestLog) -> anyhow::Result<()>;
    async fn get_request_logs(
        &self,
        tenant_id: &str,
        start_time: u64,
        end_time: u64,
        limit: u32,
    ) -> anyhow::Result<Vec<crate::RequestLog>>;
    async fn get_tenant_costs(&self, tenant_id: &str, days: u32) -> anyhow::Result<f64>;
    async fn create_webhook(&self, webhook: &crate::Webhook) -> anyhow::Result<()>;
    async fn get_webhooks(&self, tenant_id: &str) -> anyhow::Result<Vec<crate::Webhook>>;
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn validate_api_key(&self, key: &str) -> anyhow::Result<crate::Tenant>;
    async fn validate_jwt(&self, token: &str) -> anyhow::Result<(String, Vec<String>)>;
    async fn issue_jwt(&self, tenant_id: &str, scopes: Vec<&str>) -> anyhow::Result<String>;
}

#[async_trait]
pub trait RouterService: Send + Sync {
    async fn select_provider(
        &self,
        tenant_id: &str,
        model: &str,
        strategy: &str,
    ) -> anyhow::Result<String>;
    async fn record_feedback(
        &self,
        provider: &str,
        success: bool,
        latency_ms: f64,
    ) -> anyhow::Result<()>;
}

#[async_trait]
pub trait RateLimitService: Send + Sync {
    async fn check_rpm(&self, tenant_id: &str, model: &str, limit: u32) -> anyhow::Result<bool>;
    async fn check_tpm(
        &self,
        tenant_id: &str,
        model: &str,
        tokens: u32,
        limit: u32,
    ) -> anyhow::Result<bool>;
    async fn get_remaining(
        &self,
        tenant_id: &str,
        model: &str,
        rpm_limit: u32,
        tpm_limit: u32,
    ) -> anyhow::Result<(u32, u32)>;
}

#[async_trait]
pub trait BillingService: Send + Sync {
    async fn calculate_cost(
        &self,
        model: &str,
        tokens_in: u32,
        tokens_out: u32,
    ) -> anyhow::Result<f64>;
    async fn check_budget(
        &self,
        tenant_id: &str,
        additional_cost: f64,
    ) -> anyhow::Result<bool>;
    async fn record_usage(
        &self,
        tenant_id: &str,
        model: &str,
        provider: &str,
        tokens_in: u32,
        tokens_out: u32,
        latency_ms: u32,
    ) -> anyhow::Result<()>;
}

pub struct ServiceRegistry {
    services: std::sync::Arc<dashmap::DashMap<String, Arc<dyn OmnisystemService>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: std::sync::Arc::new(dashmap::DashMap::new()),
        }
    }

    pub async fn register(&self, service: Arc<dyn OmnisystemService>) -> anyhow::Result<()> {
        let service_id = service.service_id().to_string();
        self.services.insert(service_id, service);
        Ok(())
    }

    pub fn get(&self, service_id: &str) -> Option<Arc<dyn OmnisystemService>> {
        self.services.get(service_id).map(|s| s.clone())
    }

    pub fn list_services(&self) -> Vec<String> {
        self.services
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_registry() {
        let registry = ServiceRegistry::new();

        struct MockService;
        #[async_trait]
        impl OmnisystemService for MockService {
            fn service_id(&self) -> &str {
                "mock-service"
            }
            fn service_name(&self) -> &str {
                "Mock Service"
            }
            fn version(&self) -> &str {
                "1.0.0"
            }
            async fn initialize(&self) -> anyhow::Result<()> {
                Ok(())
            }
            async fn health_check(&self) -> anyhow::Result<bool> {
                Ok(true)
            }
            async fn shutdown(&self) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let service = Arc::new(MockService);
        registry.register(service.clone()).await.unwrap();

        assert!(registry.get("mock-service").is_some());
        let services = registry.list_services();
        assert!(services.contains(&"mock-service".to_string()));
    }
}
