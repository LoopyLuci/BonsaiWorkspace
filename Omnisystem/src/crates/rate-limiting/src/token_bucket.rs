use crate::{RateLimitDecision, RateLimitError, RateLimitResult, RequestPriority, TokenBucket, TokenBucketConfig};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct TokenBucketLimiter {
    buckets: Arc<DashMap<String, TokenBucket>>,
}

impl TokenBucketLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_bucket(&self, bucket_id: &str, config: &TokenBucketConfig) -> RateLimitResult<TokenBucket> {
        let bucket = TokenBucket {
            bucket_id: bucket_id.to_string(),
            tokens: config.capacity as f64,
            capacity: config.capacity,
            refill_rate: config.refill_rate,
            refill_interval_ms: config.refill_interval_ms,
            last_refill: Utc::now(),
        };

        self.buckets.insert(bucket_id.to_string(), bucket.clone());
        Ok(bucket)
    }

    pub async fn allow_request(
        &self,
        bucket_id: &str,
        tokens_needed: u64,
        priority: RequestPriority,
    ) -> RateLimitResult<RateLimitDecision> {
        if let Some(mut bucket) = self.buckets.get_mut(bucket_id) {
            self.refill_tokens(&mut bucket);

            let priority_multiplier = match priority {
                RequestPriority::Low => 1.0,
                RequestPriority::Normal => 1.0,
                RequestPriority::High => 1.5,
                RequestPriority::Critical => 2.0,
            };

            let effective_tokens = tokens_needed as f64 / priority_multiplier;

            if bucket.tokens >= effective_tokens {
                bucket.tokens -= effective_tokens;
                Ok(RateLimitDecision {
                    allowed: true,
                    tokens_remaining: bucket.tokens,
                    retry_after_ms: None,
                    priority_level: priority,
                })
            } else {
                let refill_time = self.calculate_refill_time(&bucket, effective_tokens);
                Ok(RateLimitDecision {
                    allowed: false,
                    tokens_remaining: bucket.tokens,
                    retry_after_ms: Some(refill_time),
                    priority_level: priority,
                })
            }
        } else {
            Err(RateLimitError::BucketNotFound)
        }
    }

    fn refill_tokens(&self, bucket: &mut TokenBucket) {
        let now = Utc::now();
        let elapsed = now
            .signed_duration_since(bucket.last_refill)
            .num_milliseconds() as u64;

        if elapsed >= bucket.refill_interval_ms {
            let intervals = elapsed / bucket.refill_interval_ms;
            let tokens_to_add = (intervals as u64 * bucket.refill_rate) as f64;
            bucket.tokens = (bucket.tokens + tokens_to_add).min(bucket.capacity as f64);
            bucket.last_refill = now;
        }
    }

    fn calculate_refill_time(&self, bucket: &TokenBucket, tokens_needed: f64) -> u64 {
        let tokens_deficit = tokens_needed - bucket.tokens;
        let intervals_needed = (tokens_deficit / bucket.refill_rate as f64).ceil() as u64;
        intervals_needed * bucket.refill_interval_ms
    }

    pub async fn get_bucket(&self, bucket_id: &str) -> RateLimitResult<TokenBucket> {
        self.buckets
            .get(bucket_id)
            .map(|entry| entry.clone())
            .ok_or(RateLimitError::BucketNotFound)
    }

    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

impl Default for TokenBucketLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_bucket() {
        let limiter = TokenBucketLimiter::new();
        let config = TokenBucketConfig {
            capacity: 100,
            refill_rate: 10,
            refill_interval_ms: 1000,
        };

        let bucket = limiter.create_bucket("bucket-1", &config).await.unwrap();
        assert_eq!(bucket.bucket_id, "bucket-1");
        assert_eq!(bucket.tokens, 100.0);
    }

    #[tokio::test]
    async fn test_allow_request_success() {
        let limiter = TokenBucketLimiter::new();
        let config = TokenBucketConfig {
            capacity: 100,
            refill_rate: 10,
            refill_interval_ms: 1000,
        };

        limiter.create_bucket("bucket-1", &config).await.unwrap();
        let decision = limiter
            .allow_request("bucket-1", 50, RequestPriority::Normal)
            .await
            .unwrap();

        assert!(decision.allowed);
        assert_eq!(decision.tokens_remaining, 50.0);
    }

    #[tokio::test]
    async fn test_allow_request_exceeds_capacity() {
        let limiter = TokenBucketLimiter::new();
        let config = TokenBucketConfig {
            capacity: 100,
            refill_rate: 10,
            refill_interval_ms: 1000,
        };

        limiter.create_bucket("bucket-1", &config).await.unwrap();
        let decision = limiter
            .allow_request("bucket-1", 150, RequestPriority::Normal)
            .await
            .unwrap();

        assert!(!decision.allowed);
        assert!(decision.retry_after_ms.is_some());
    }

    #[tokio::test]
    async fn test_priority_multiplier() {
        let limiter = TokenBucketLimiter::new();
        let config = TokenBucketConfig {
            capacity: 100,
            refill_rate: 10,
            refill_interval_ms: 1000,
        };

        limiter.create_bucket("bucket-1", &config).await.unwrap();

        let decision_high = limiter
            .allow_request("bucket-1", 50, RequestPriority::High)
            .await
            .unwrap();

        let decision_critical = limiter
            .allow_request("bucket-1", 50, RequestPriority::Critical)
            .await
            .unwrap();

        assert!(decision_high.allowed);
        assert!(decision_critical.allowed);
    }

    #[tokio::test]
    async fn test_get_bucket() {
        let limiter = TokenBucketLimiter::new();
        let config = TokenBucketConfig {
            capacity: 100,
            refill_rate: 10,
            refill_interval_ms: 1000,
        };

        limiter.create_bucket("bucket-1", &config).await.unwrap();
        let bucket = limiter.get_bucket("bucket-1").await.unwrap();

        assert_eq!(bucket.capacity, 100);
    }

    #[tokio::test]
    async fn test_bucket_not_found() {
        let limiter = TokenBucketLimiter::new();
        let result = limiter.allow_request("nonexistent", 10, RequestPriority::Normal).await;

        assert!(result.is_err());
    }
}
