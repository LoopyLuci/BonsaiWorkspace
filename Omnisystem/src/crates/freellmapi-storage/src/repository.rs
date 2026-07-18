use freellmapi_core::*;
use async_trait::async_trait;
use crate::StorageManager;

#[async_trait]
impl StorageRepository for StorageManager {
    async fn create_tenant(&self, tenant: &Tenant) -> anyhow::Result<()> {
        StorageManager::create_tenant(self, tenant).await
    }

    async fn get_tenant(&self, id: &str) -> anyhow::Result<Tenant> {
        StorageManager::get_tenant(self, id).await
    }

    async fn create_api_key(&self, key: &ApiKey) -> anyhow::Result<()> {
        StorageManager::create_api_key(self, key).await
    }

    async fn get_api_key_by_hash(&self, hash: &str) -> anyhow::Result<ApiKey> {
        StorageManager::get_api_key_by_hash(self, hash).await
    }

    async fn log_request(&self, log: &RequestLog) -> anyhow::Result<()> {
        StorageManager::log_request(self, log).await
    }

    async fn get_request_logs(
        &self,
        tenant_id: &str,
        start_time: u64,
        end_time: u64,
        limit: u32,
    ) -> anyhow::Result<Vec<RequestLog>> {
        StorageManager::get_request_logs(self, tenant_id, start_time, end_time, limit).await
    }

    async fn get_tenant_costs(&self, tenant_id: &str, days: u32) -> anyhow::Result<f64> {
        StorageManager::get_tenant_costs(self, tenant_id, days).await
    }

    async fn create_webhook(&self, webhook: &Webhook) -> anyhow::Result<()> {
        StorageManager::create_webhook(self, webhook).await
    }

    async fn get_webhooks(&self, tenant_id: &str) -> anyhow::Result<Vec<Webhook>> {
        StorageManager::get_webhooks(self, tenant_id).await
    }
}
