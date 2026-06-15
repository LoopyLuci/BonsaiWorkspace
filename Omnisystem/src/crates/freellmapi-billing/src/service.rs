use freellmapi_core::*;
use async_trait::async_trait;
use anyhow::Result;
use std::sync::Arc;
use dashmap::DashMap;

pub struct BillingService {
    storage: Arc<dyn StorageRepository>,
    usage_cache: Arc<DashMap<String, f64>>,
}

// Pricing map: model -> (input_cost_per_1k, output_cost_per_1k)
fn get_pricing(model: &str) -> (f64, f64) {
    match model {
        "gpt-4" => (0.03, 0.06),
        "gpt-4-turbo" => (0.01, 0.03),
        "gpt-3.5-turbo" => (0.0005, 0.0015),
        "claude-3-opus" => (0.015, 0.075),
        "claude-3-sonnet" => (0.003, 0.015),
        "claude-3-haiku" => (0.00025, 0.00125),
        "llama-2-70b" => (0.0, 0.0), // Free via Groq
        "mistral-7b" => (0.0, 0.0), // Free via Groq
        _ => (0.001, 0.001), // Default estimate
    }
}

impl BillingService {
    pub async fn new(storage: Arc<dyn StorageRepository>) -> Result<Self> {
        Ok(BillingService {
            storage,
            usage_cache: Arc::new(DashMap::new()),
        })
    }

    pub async fn calculate_cost(
        &self,
        model: &str,
        tokens_in: u32,
        tokens_out: u32,
    ) -> Result<f64> {
        let (input_price, output_price) = get_pricing(model);
        let cost = (tokens_in as f64 * input_price + tokens_out as f64 * output_price) / 1000.0;
        Ok(cost)
    }

    pub async fn check_budget(
        &self,
        tenant_id: &str,
        additional_cost: f64,
    ) -> Result<bool> {
        let tenant = self.storage.get_tenant(tenant_id).await?;
        let spent_this_month = self.storage.get_tenant_costs(tenant_id, 30).await?;

        Ok(spent_this_month + additional_cost <= tenant.monthly_budget_usd)
    }

    pub async fn record_usage(
        &self,
        tenant_id: &str,
        model: &str,
        provider: &str,
        tokens_in: u32,
        tokens_out: u32,
        latency_ms: u32,
    ) -> Result<()> {
        let cost = self.calculate_cost(model, tokens_in, tokens_out).await?;

        let log = RequestLog {
            id: generate_id(),
            tenant_id: tenant_id.to_string(),
            api_key_id: "unknown".to_string(),
            model: model.to_string(),
            provider: provider.to_string(),
            tokens_in,
            tokens_out,
            cost_usd: cost,
            latency_ms,
            status_code: 200,
            created_at: unix_now(),
        };

        self.storage.log_request(&log).await?;

        // Update cache for quick access
        let day_timestamp = (unix_now() / 86400) * 86400; // Round to day boundary
        let cache_key = format!("{}-{}", tenant_id, day_timestamp);
        let current = self.usage_cache.get(&cache_key).map(|r| *r).unwrap_or(0.0);
        self.usage_cache.insert(cache_key, current + cost);

        Ok(())
    }

    pub async fn get_tenant_costs(
        &self,
        tenant_id: &str,
        days: u32,
    ) -> Result<f64> {
        self.storage.get_tenant_costs(tenant_id, days).await
    }

    pub async fn forecast_monthly_cost(&self, tenant_id: &str) -> Result<f64> {
        // Get costs from last 7 days
        let past_7_days = self.storage.get_tenant_costs(tenant_id, 7).await?;

        // Calculate daily average
        let daily_avg = if past_7_days > 0.0 {
            past_7_days / 7.0
        } else {
            0.0
        };

        // Project to monthly
        let forecast = daily_avg * 30.0;
        Ok(forecast)
    }

    pub async fn warn_if_exceeds_threshold(
        &self,
        tenant_id: &str,
        threshold_percentage: f64,
    ) -> Result<bool> {
        let tenant = self.storage.get_tenant(tenant_id).await?;
        let spent = self.storage.get_tenant_costs(tenant_id, 30).await?;
        let budget = tenant.monthly_budget_usd;

        let usage_percentage = (spent / budget) * 100.0;
        Ok(usage_percentage >= threshold_percentage)
    }

    pub async fn reset_monthly_costs(&self, _tenant_id: &str) -> Result<()> {
        // In a real system, this would mark a new billing period
        // For now, costs are naturally reset by querying only recent data
        Ok(())
    }
}

#[async_trait]
impl OmnisystemService for BillingService {
    fn service_id(&self) -> &str {
        "freellmapi-billing"
    }

    fn service_name(&self) -> &str {
        "FreeLLMAPI Billing"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    async fn initialize(&self) -> Result<()> {
        tracing::info!("Billing service initialized");
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    async fn shutdown(&self) -> Result<()> {
        self.usage_cache.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pricing() {
        let (input, output) = get_pricing("gpt-4");
        assert_eq!(input, 0.03);
        assert_eq!(output, 0.06);

        let (input, output) = get_pricing("gpt-3.5-turbo");
        assert_eq!(input, 0.0005);
        assert_eq!(output, 0.0015);

        let (input, output) = get_pricing("unknown-model");
        assert_eq!(input, 0.001);
        assert_eq!(output, 0.001);
    }

    #[test]
    fn test_cost_formula() {
        let (input_price, output_price) = get_pricing("gpt-4");
        let tokens_in = 1000u32;
        let tokens_out = 500u32;

        let cost = (tokens_in as f64 * input_price + tokens_out as f64 * output_price) / 1000.0;
        let expected = (1000.0 * 0.03 + 500.0 * 0.06) / 1000.0;

        assert!((cost - expected).abs() < 0.0001);
    }
}
