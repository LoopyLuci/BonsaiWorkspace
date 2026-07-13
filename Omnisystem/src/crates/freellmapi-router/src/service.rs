use freellmapi_core::*;
use async_trait::async_trait;
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::sync::Arc;
use crate::BetaDistribution;

pub struct RouterService {
    metrics: Arc<DashMap<String, ProviderMetrics>>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ProviderStats {
    pub name: String,
    pub beta: BetaDistribution,
    pub avg_latency_ms: f64,
    pub cost_per_1k: f64,
}

impl RouterService {
    pub async fn new() -> Result<Self> {
        Ok(RouterService {
            metrics: Arc::new(DashMap::new()),
        })
    }

    pub async fn register_provider(
        &self,
        provider: &str,
        alpha: f64,
        beta: f64,
        latency: f64,
        cost: f64,
    ) -> Result<()> {
        self.metrics.insert(
            provider.to_string(),
            ProviderMetrics {
                name: provider.to_string(),
                alpha,
                beta,
                avg_latency_ms: latency,
                cost_per_1k_tokens: cost,
            },
        );
        Ok(())
    }

    pub async fn select_provider(
        &self,
        _model: &str,
        strategy: &str,
    ) -> Result<String> {
        if self.metrics.is_empty() {
            return Err(anyhow!("No providers registered"));
        }

        match strategy {
            "balanced" => self.select_balanced().await,
            "fastest" => self.select_fastest().await,
            "cheapest" => self.select_cheapest().await,
            "reliable" => self.select_reliable().await,
            _ => Err(anyhow!("Unknown strategy: {}", strategy)),
        }
    }

    async fn select_balanced(&self) -> Result<String> {
        let mut best_provider = String::new();
        let mut best_score = -1.0;

        for entry in self.metrics.iter() {
            let provider_name = entry.key().clone();
            let metrics = entry.value().clone();

            // Thompson Sampling: sample from Beta distribution
            let reliability_sample = {
                let dist = BetaDistribution::new(metrics.alpha, metrics.beta);
                dist.sample()
            };

            // Normalize latency (faster = higher score)
            let latency_score = 1.0 / (1.0 + metrics.avg_latency_ms / 100.0);

            // Normalize cost (cheaper = higher score)
            let cost_score = 1.0 / (1.0 + metrics.cost_per_1k_tokens * 1000.0);

            // Weighted score: 50% reliability, 30% speed, 20% cost
            let score = reliability_sample * 0.5 + latency_score * 0.3 + cost_score * 0.2;

            if score > best_score {
                best_score = score;
                best_provider = provider_name;
            }
        }

        Ok(best_provider)
    }

    async fn select_fastest(&self) -> Result<String> {
        let provider = self.metrics
            .iter()
            .min_by(|a, b| {
                a.value()
                    .avg_latency_ms
                    .partial_cmp(&b.value().avg_latency_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| anyhow!("No providers available"))?;

        Ok(provider.key().clone())
    }

    async fn select_cheapest(&self) -> Result<String> {
        let provider = self.metrics
            .iter()
            .min_by(|a, b| {
                a.value()
                    .cost_per_1k_tokens
                    .partial_cmp(&b.value().cost_per_1k_tokens)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| anyhow!("No providers available"))?;

        Ok(provider.key().clone())
    }

    async fn select_reliable(&self) -> Result<String> {
        let provider = self.metrics
            .iter()
            .max_by(|a, b| {
                a.value()
                    .alpha
                    .partial_cmp(&b.value().alpha)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| anyhow!("No providers available"))?;

        Ok(provider.key().clone())
    }

    pub async fn record_feedback(
        &self,
        provider: &str,
        success: bool,
        latency_ms: f64,
    ) -> Result<()> {
        if let Some(mut metrics) = self.metrics.get_mut(provider) {
            if success {
                metrics.alpha += 1.0;
            } else {
                metrics.beta += 1.0;
            }

            // Exponential moving average for latency
            metrics.avg_latency_ms = metrics.avg_latency_ms * 0.9 + latency_ms * 0.1;
        }

        Ok(())
    }

    pub async fn get_provider_stats(&self, provider: &str) -> Result<Option<ProviderMetrics>> {
        Ok(self.metrics.get(provider).map(|entry| entry.clone()))
    }

    pub async fn list_providers(&self) -> Result<Vec<String>> {
        Ok(self.metrics
            .iter()
            .map(|entry| entry.key().clone())
            .collect())
    }
}

#[async_trait]
impl OmnisystemService for RouterService {
    fn service_id(&self) -> &str {
        "freellmapi-router"
    }

    fn service_name(&self) -> &str {
        "FreeLLMAPI Router"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    async fn initialize(&self) -> Result<()> {
        tracing::info!("Router service initialized");
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(!self.metrics.is_empty())
    }

    async fn shutdown(&self) -> Result<()> {
        self.metrics.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_latency_exponential_average() {
        let router = RouterService::new().await.unwrap();
        router.register_provider("test", 1.0, 1.0, 100.0, 0.0).await.unwrap();

        router.record_feedback("test", true, 150.0).await.unwrap();
        let stats = router.get_provider_stats("test").await.unwrap().unwrap();

        // avg = 100.0 * 0.9 + 150.0 * 0.1 = 90 + 15 = 105
        assert!((stats.avg_latency_ms - 105.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_multiple_providers() {
        let router = RouterService::new().await.unwrap();
        router.register_provider("openai", 1.0, 1.0, 100.0, 0.002).await.unwrap();
        router.register_provider("groq", 1.0, 1.0, 50.0, 0.0).await.unwrap();
        router.register_provider("cerebras", 1.0, 1.0, 200.0, 0.0).await.unwrap();

        let providers = router.list_providers().await.unwrap();
        assert_eq!(providers.len(), 3);
    }

    #[tokio::test]
    async fn test_strategy_consistency() {
        let router = RouterService::new().await.unwrap();
        router.register_provider("openai", 5.0, 1.0, 100.0, 0.002).await.unwrap();
        router.register_provider("groq", 1.0, 5.0, 50.0, 0.0).await.unwrap();

        // Fastest should pick Groq
        let fastest = router.select_provider("gpt-4", "fastest").await.unwrap();
        assert_eq!(fastest, "groq");

        // Cheapest should pick Groq
        let cheapest = router.select_provider("gpt-4", "cheapest").await.unwrap();
        assert_eq!(cheapest, "groq");

        // Reliable should pick OpenAI
        let reliable = router.select_provider("gpt-4", "reliable").await.unwrap();
        assert_eq!(reliable, "openai");
    }
}
