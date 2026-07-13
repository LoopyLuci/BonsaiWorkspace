use crate::{MeshResult, RateLimitConfig, ServiceId};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct TokenBucket {
    tokens: f64,
    last_refill: i64,
    capacity: f64,
    refill_rate: f64,
}

pub struct RateLimiter {
    buckets: Arc<DashMap<String, TokenBucket>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            config,
        }
    }

    pub async fn check_rate_limit(&self, service_id: &ServiceId) -> MeshResult<bool> {
        self.acquire_token(service_id, 1).await
    }

    pub async fn acquire_token(&self, service_id: &ServiceId, count: u32) -> MeshResult<bool> {
        let now = Utc::now().timestamp_millis();
        let count_f = count as f64;

        if let Some(mut bucket) = self.buckets.get_mut(&service_id.0) {
            let elapsed_ms = now - bucket.last_refill;
            let tokens_to_add = (elapsed_ms as f64 / 1000.0) * bucket.refill_rate;
            bucket.tokens = (bucket.tokens + tokens_to_add).min(bucket.capacity);
            bucket.last_refill = now;

            if bucket.tokens >= count_f {
                bucket.tokens -= count_f;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            let capacity = self.config.burst_size as f64;
            let mut bucket = TokenBucket {
                tokens: capacity,
                last_refill: now,
                capacity,
                refill_rate: self.config.requests_per_second as f64,
            };

            if bucket.tokens >= count_f {
                bucket.tokens -= count_f;
                self.buckets.insert(service_id.0.clone(), bucket);
                Ok(true)
            } else {
                self.buckets.insert(service_id.0.clone(), bucket);
                Ok(false)
            }
        }
    }

    pub async fn reset_limits(&self, service_id: &ServiceId) -> MeshResult<()> {
        if let Some(mut bucket) = self.buckets.get_mut(&service_id.0) {
            bucket.tokens = bucket.capacity;
            bucket.last_refill = Utc::now().timestamp_millis();
        }
        Ok(())
    }

    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_token_within_limit() {
        let config = RateLimitConfig {
            requests_per_second: 10,
            burst_size: 5,
        };
        let limiter = RateLimiter::new(config);
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        let result = limiter.acquire_token(&service_id, 3).await;
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_acquire_token_exceeds_burst() {
        let config = RateLimitConfig {
            requests_per_second: 10,
            burst_size: 5,
        };
        let limiter = RateLimiter::new(config);
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        let result = limiter.acquire_token(&service_id, 10).await;
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_check_rate_limit() {
        let config = RateLimitConfig {
            requests_per_second: 100,
            burst_size: 50,
        };
        let limiter = RateLimiter::new(config);
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        let result = limiter.check_rate_limit(&service_id).await;
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_rate_limit_exhaustion() {
        let config = RateLimitConfig {
            requests_per_second: 100,
            burst_size: 3,
        };
        let limiter = RateLimiter::new(config);
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        assert!(limiter.acquire_token(&service_id, 1).await.unwrap());
        assert!(limiter.acquire_token(&service_id, 1).await.unwrap());
        assert!(limiter.acquire_token(&service_id, 1).await.unwrap());
        assert!(!limiter.acquire_token(&service_id, 1).await.unwrap());
    }

    #[tokio::test]
    async fn test_reset_limits() {
        let config = RateLimitConfig {
            requests_per_second: 10,
            burst_size: 3,
        };
        let limiter = RateLimiter::new(config);
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        limiter.acquire_token(&service_id, 3).await.unwrap();
        assert!(!limiter.acquire_token(&service_id, 1).await.unwrap());

        limiter.reset_limits(&service_id).await.unwrap();
        assert!(limiter.acquire_token(&service_id, 1).await.unwrap());
    }
}
