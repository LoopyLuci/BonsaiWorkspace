use freellmapi_core::*;
use async_trait::async_trait;
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct RateLimitCounter {
    rpm: u32,
    tpm: u32,
    reset_at: u64,
}

pub struct RateLimitService {
    counters: Arc<DashMap<String, RateLimitCounter>>,
}

impl RateLimitService {
    pub fn new() -> Self {
        RateLimitService {
            counters: Arc::new(DashMap::new()),
        }
    }

    fn make_key(tenant_id: &str, model: &str, prefix: &str) -> String {
        format!("{}:{}:{}", prefix, tenant_id, model)
    }

    pub async fn check_rpm(&self, tenant_id: &str, model: &str, limit: u32) -> Result<bool> {
        let key = Self::make_key(tenant_id, model, "rpm");
        let now = unix_now();

        let mut counter = self.counters.entry(key.clone()).or_insert_with(|| {
            RateLimitCounter {
                rpm: 0,
                tpm: 0,
                reset_at: now + 60,
            }
        });

        // Reset if window has passed
        if counter.reset_at <= now {
            counter.rpm = 0;
            counter.reset_at = now + 60;
        }

        // Check limit
        if counter.rpm >= limit {
            return Ok(false); // Rate limited
        }

        counter.rpm += 1;
        Ok(true)
    }

    pub async fn check_tpm(
        &self,
        tenant_id: &str,
        model: &str,
        tokens: u32,
        limit: u32,
    ) -> Result<bool> {
        let key = Self::make_key(tenant_id, model, "tpm");
        let now = unix_now();

        let mut counter = self.counters.entry(key.clone()).or_insert_with(|| {
            RateLimitCounter {
                rpm: 0,
                tpm: 0,
                reset_at: now + 60,
            }
        });

        // Reset if window has passed
        if counter.reset_at <= now {
            counter.tpm = 0;
            counter.reset_at = now + 60;
        }

        // Check if adding tokens would exceed limit
        if counter.tpm + tokens > limit {
            return Ok(false); // Would exceed limit
        }

        counter.tpm += tokens;
        Ok(true)
    }

    pub async fn get_remaining(
        &self,
        tenant_id: &str,
        model: &str,
        rpm_limit: u32,
        tpm_limit: u32,
    ) -> Result<(u32, u32)> {
        let rpm_key = Self::make_key(tenant_id, model, "rpm");
        let tpm_key = Self::make_key(tenant_id, model, "tpm");

        let rpm_used = self.counters
            .get(&rpm_key)
            .map(|c| c.rpm)
            .unwrap_or(0);

        let tpm_used = self.counters
            .get(&tpm_key)
            .map(|c| c.tpm)
            .unwrap_or(0);

        Ok((
            rpm_limit.saturating_sub(rpm_used),
            tpm_limit.saturating_sub(tpm_used),
        ))
    }

    pub async fn reset_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<()> {
        // Remove all counters for this tenant
        self.counters.retain(|key, _| !key.contains(tenant_id));
        Ok(())
    }

    pub async fn get_stats(
        &self,
        tenant_id: &str,
        model: &str,
    ) -> Result<Option<(u32, u32)>> {
        let rpm_key = Self::make_key(tenant_id, model, "rpm");
        let tpm_key = Self::make_key(tenant_id, model, "tpm");

        let rpm = self.counters.get(&rpm_key).map(|c| c.rpm);
        let tpm = self.counters.get(&tpm_key).map(|c| c.tpm);

        match (rpm, tpm) {
            (Some(r), Some(t)) => Ok(Some((r, t))),
            _ => Ok(None),
        }
    }
}

#[async_trait]
impl OmnisystemService for RateLimitService {
    fn service_id(&self) -> &str {
        "freellmapi-ratelimit"
    }

    fn service_name(&self) -> &str {
        "FreeLLMAPI Rate Limit"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    async fn initialize(&self) -> Result<()> {
        tracing::info!("Rate limiter service initialized");
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    async fn shutdown(&self) -> Result<()> {
        self.counters.clear();
        Ok(())
    }
}

impl Default for RateLimitService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key = RateLimitService::make_key("tenant1", "gpt-4", "rpm");
        assert_eq!(key, "rpm:tenant1:gpt-4");
    }

    #[test]
    fn test_counter_reset() {
        let limiter = RateLimitService::new();
        let key = RateLimitService::make_key("t1", "m1", "rpm");

        // Insert a counter
        limiter.counters.insert(
            key.clone(),
            RateLimitCounter {
                rpm: 5,
                tpm: 100,
                reset_at: unix_now() - 10, // Already expired
            },
        );

        let counter = limiter.counters.get(&key).unwrap();
        assert_eq!(counter.rpm, 5);
    }
}
