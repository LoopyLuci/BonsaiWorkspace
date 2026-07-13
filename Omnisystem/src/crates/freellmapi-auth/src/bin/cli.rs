//! Demo CLI: issues and validates a real HMAC-signed JWT. The signing secret is
//! read from FREELLMAPI_AUTH_SECRET (falling back to a dev-only default) - never
//! hardcode a production secret here.

use async_trait::async_trait;
use freellmapi_core::{ApiKey, RequestLog, StorageRepository, Tenant, Webhook};
use freellmapi_auth::AuthService;
use std::sync::Arc;

struct NullStorage;

#[async_trait]
impl StorageRepository for NullStorage {
    async fn create_tenant(&self, _: &Tenant) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_tenant(&self, id: &str) -> anyhow::Result<Tenant> {
        Ok(Tenant {
            id: id.to_string(),
            name: "CLI Demo Tenant".to_string(),
            email: "demo@example.com".to_string(),
            tier: "free".to_string(),
            monthly_budget_usd: 10.0,
            created_at: 0,
        })
    }
    async fn create_api_key(&self, _: &ApiKey) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_api_key_by_hash(&self, _: &str) -> anyhow::Result<ApiKey> {
        Ok(ApiKey {
            id: "demo-key".to_string(),
            tenant_id: "demo-tenant".to_string(),
            key_hash: "demo-hash".to_string(),
            scopes: vec![],
            created_at: 0,
            expires_at: None,
        })
    }
    async fn log_request(&self, _: &RequestLog) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_request_logs(&self, _: &str, _: u64, _: u64, _: u32) -> anyhow::Result<Vec<RequestLog>> {
        Ok(vec![])
    }
    async fn get_tenant_costs(&self, _: &str, _: u32) -> anyhow::Result<f64> {
        Ok(0.0)
    }
    async fn create_webhook(&self, _: &Webhook) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_webhooks(&self, _: &str) -> anyhow::Result<Vec<Webhook>> {
        Ok(vec![])
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let secret = std::env::var("FREELLMAPI_AUTH_SECRET").unwrap_or_else(|_| "dev-only-secret".to_string());
    let auth = AuthService::new(Arc::new(NullStorage), &secret)?;

    let token = auth.issue_jwt("demo-tenant", vec!["chat", "models"]).await?;
    println!("Issued JWT: {token}");

    let (subject, scopes) = auth.validate_jwt(&token).await?;
    println!("Validated JWT for subject={subject} scopes={scopes:?}");

    Ok(())
}
