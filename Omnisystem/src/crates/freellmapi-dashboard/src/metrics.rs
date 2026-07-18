use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct DashboardMetrics {
    tenant_metrics: Arc<DashMap<String, TenantMetrics>>,
    provider_metrics: Arc<DashMap<String, ProviderMetrics>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMetrics {
    pub tenant_id: String,
    pub total_requests: u64,
    pub total_cost_usd: f64,
    pub total_latency_ms: u64,
    pub avg_latency_ms: f64,
    pub last_request_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub provider: String,
    pub total_requests: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub total_cost_usd: f64,
}

impl DashboardMetrics {
    pub fn new() -> Self {
        DashboardMetrics {
            tenant_metrics: Arc::new(DashMap::new()),
            provider_metrics: Arc::new(DashMap::new()),
        }
    }

    pub async fn record_request(&self, tenant_id: &str, provider: &str, cost: f64, latency_ms: u32) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Update tenant metrics
        self.tenant_metrics
            .entry(tenant_id.to_string())
            .and_modify(|m| {
                m.total_requests += 1;
                m.total_cost_usd += cost;
                m.total_latency_ms += latency_ms as u64;
                m.avg_latency_ms = m.total_latency_ms as f64 / m.total_requests as f64;
                m.last_request_at = now;
            })
            .or_insert_with(|| TenantMetrics {
                tenant_id: tenant_id.to_string(),
                total_requests: 1,
                total_cost_usd: cost,
                total_latency_ms: latency_ms as u64,
                avg_latency_ms: latency_ms as f64,
                last_request_at: now,
            });

        // Update provider metrics
        self.provider_metrics
            .entry(provider.to_string())
            .and_modify(|m| {
                let prev_avg = m.avg_latency_ms;
                m.total_requests += 1;
                m.total_cost_usd += cost;
                m.avg_latency_ms = (prev_avg * (m.total_requests - 1) as f64 + latency_ms as f64) / m.total_requests as f64;
            })
            .or_insert_with(|| ProviderMetrics {
                provider: provider.to_string(),
                total_requests: 1,
                success_rate: 100.0,
                avg_latency_ms: latency_ms as f64,
                total_cost_usd: cost,
            });
    }

    pub async fn get_tenant_metrics(&self, tenant_id: &str) -> Result<TenantMetrics> {
        if let Some(m) = self.tenant_metrics.get(tenant_id) {
            Ok(m.value().clone())
        } else {
            Ok(TenantMetrics {
                tenant_id: tenant_id.to_string(),
                total_requests: 0,
                total_cost_usd: 0.0,
                total_latency_ms: 0,
                avg_latency_ms: 0.0,
                last_request_at: 0,
            })
        }
    }

    pub async fn get_provider_metrics(&self, provider: &str) -> Result<ProviderMetrics> {
        if let Some(m) = self.provider_metrics.get(provider) {
            Ok(m.value().clone())
        } else {
            Ok(ProviderMetrics {
                provider: provider.to_string(),
                total_requests: 0,
                success_rate: 0.0,
                avg_latency_ms: 0.0,
                total_cost_usd: 0.0,
            })
        }
    }

    pub async fn get_all_metrics(&self) -> Result<(Vec<TenantMetrics>, Vec<ProviderMetrics>)> {
        let tenants = self.tenant_metrics
            .iter()
            .map(|r| r.value().clone())
            .collect();

        let providers = self.provider_metrics
            .iter()
            .map(|r| r.value().clone())
            .collect();

        Ok((tenants, providers))
    }
}

impl Default for DashboardMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_and_retrieve() {
        let metrics = DashboardMetrics::new();
        metrics.record_request("tenant1", "openai", 0.05, 250).await;

        let tenant = metrics.get_tenant_metrics("tenant1").await.unwrap();
        assert_eq!(tenant.total_requests, 1);
        assert_eq!(tenant.total_cost_usd, 0.05);

        let provider = metrics.get_provider_metrics("openai").await.unwrap();
        assert_eq!(provider.total_requests, 1);
    }

    #[tokio::test]
    async fn test_multiple_requests() {
        let metrics = DashboardMetrics::new();
        metrics.record_request("tenant1", "openai", 0.05, 100).await;
        metrics.record_request("tenant1", "openai", 0.06, 200).await;
        metrics.record_request("tenant1", "openai", 0.04, 300).await;

        let tenant = metrics.get_tenant_metrics("tenant1").await.unwrap();
        assert_eq!(tenant.total_requests, 3);
        assert_eq!(tenant.total_cost_usd, 0.15);
    }
}
