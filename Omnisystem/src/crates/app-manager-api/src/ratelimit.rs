//! Rate limiting middleware

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Duration, Utc};

/// Request count with timestamp
#[derive(Debug, Clone)]
struct RequestBucket {
    count: u32,
    reset_at: DateTime<Utc>,
}

/// Rate limiter for request throttling
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, RequestBucket>>>,
    max_requests: u32,
    window_seconds: i64,
}

impl RateLimiter {
    /// Create new rate limiter
    /// max_requests: Maximum requests per window
    /// window_seconds: Time window in seconds
    pub fn new(max_requests: u32, window_seconds: i64) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_seconds,
        }
    }

    /// Standard limits: 100 requests per minute
    pub fn standard() -> Self {
        Self::new(100, 60)
    }

    /// Strict limits: 10 requests per minute (for login)
    pub fn strict() -> Self {
        Self::new(10, 60)
    }

    /// Relaxed limits: 1000 requests per minute
    pub fn relaxed() -> Self {
        Self::new(1000, 60)
    }

    /// Check if request is allowed for client
    pub fn allow_request(&self, client_id: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Utc::now();

        let bucket = buckets
            .entry(client_id.to_string())
            .or_insert_with(|| RequestBucket {
                count: 0,
                reset_at: now + Duration::seconds(self.window_seconds),
            });

        // Reset bucket if window expired
        if now >= bucket.reset_at {
            bucket.count = 0;
            bucket.reset_at = now + Duration::seconds(self.window_seconds);
        }

        // Increment counter
        bucket.count += 1;

        // Allow if under limit
        bucket.count <= self.max_requests
    }

    /// Get remaining requests for client
    pub fn remaining_requests(&self, client_id: &str) -> u32 {
        let buckets = self.buckets.lock().unwrap();
        let now = Utc::now();

        if let Some(bucket) = buckets.get(client_id) {
            if now < bucket.reset_at {
                return self.max_requests.saturating_sub(bucket.count);
            }
        }

        self.max_requests
    }

    /// Get reset time for client
    pub fn reset_at(&self, client_id: &str) -> Option<DateTime<Utc>> {
        let buckets = self.buckets.lock().unwrap();
        buckets.get(client_id).map(|b| b.reset_at)
    }

    /// Clear all buckets (for testing)
    pub fn clear(&self) {
        let mut buckets = self.buckets.lock().unwrap();
        buckets.clear();
    }

    /// Get bucket info for monitoring
    pub fn get_bucket_info(&self, client_id: &str) -> Option<(u32, DateTime<Utc>)> {
        let buckets = self.buckets.lock().unwrap();
        buckets.get(client_id).map(|b| (b.count, b.reset_at))
    }
}

/// Rate limit headers builder
pub struct RateLimitHeaders {
    pub limit: u32,
    pub remaining: u32,
    pub reset: i64,
}

impl RateLimitHeaders {
    /// Create headers from rate limiter
    pub fn from_limiter(limiter: &RateLimiter, client_id: &str) -> Self {
        let remaining = limiter.remaining_requests(client_id);
        let reset = limiter
            .reset_at(client_id)
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| (Utc::now() + Duration::seconds(60)).timestamp());

        Self {
            limit: limiter.max_requests,
            remaining,
            reset,
        }
    }

    /// Get X-RateLimit-Limit header value
    pub fn limit_header(&self) -> String {
        self.limit.to_string()
    }

    /// Get X-RateLimit-Remaining header value
    pub fn remaining_header(&self) -> String {
        self.remaining.to_string()
    }

    /// Get X-RateLimit-Reset header value
    pub fn reset_header(&self) -> String {
        self.reset.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_standard() {
        let limiter = RateLimiter::standard();
        assert_eq!(limiter.max_requests, 100);
    }

    #[test]
    fn test_rate_limiter_strict() {
        let limiter = RateLimiter::strict();
        assert_eq!(limiter.max_requests, 10);
    }

    #[test]
    fn test_allow_requests_under_limit() {
        let limiter = RateLimiter::new(5, 60);
        let client = "client-1";

        for i in 1..=5 {
            assert!(limiter.allow_request(client), "Request {} should be allowed", i);
        }
    }

    #[test]
    fn test_deny_requests_over_limit() {
        let limiter = RateLimiter::new(3, 60);
        let client = "client-1";

        // Allow first 3
        assert!(limiter.allow_request(client));
        assert!(limiter.allow_request(client));
        assert!(limiter.allow_request(client));

        // Deny 4th
        assert!(!limiter.allow_request(client));
    }

    #[test]
    fn test_remaining_requests() {
        let limiter = RateLimiter::new(10, 60);
        let client = "client-1";

        assert_eq!(limiter.remaining_requests(client), 10);

        limiter.allow_request(client);
        assert_eq!(limiter.remaining_requests(client), 9);

        limiter.allow_request(client);
        assert_eq!(limiter.remaining_requests(client), 8);
    }

    #[test]
    fn test_multiple_clients_isolated() {
        let limiter = RateLimiter::new(5, 60);

        // Client 1 uses 3 requests
        limiter.allow_request("client-1");
        limiter.allow_request("client-1");
        limiter.allow_request("client-1");

        // Client 2 still has 5 available
        assert_eq!(limiter.remaining_requests("client-2"), 5);

        // Client 1 has 2 remaining
        assert_eq!(limiter.remaining_requests("client-1"), 2);
    }

    #[test]
    fn test_rate_limit_headers() {
        let limiter = RateLimiter::new(100, 60);
        let client = "client-1";

        limiter.allow_request(client);
        limiter.allow_request(client);

        let headers = RateLimitHeaders::from_limiter(&limiter, client);
        assert_eq!(headers.limit, 100);
        assert_eq!(headers.remaining, 98);
        assert!(headers.reset > 0);
    }

    #[test]
    fn test_clear_buckets() {
        let limiter = RateLimiter::new(5, 60);
        let client = "client-1";

        limiter.allow_request(client);
        limiter.allow_request(client);
        assert_eq!(limiter.remaining_requests(client), 3);

        limiter.clear();
        assert_eq!(limiter.remaining_requests(client), 5);
    }

    #[test]
    fn test_bucket_info() {
        let limiter = RateLimiter::new(5, 60);
        let client = "client-1";

        assert!(limiter.get_bucket_info(client).is_none());

        limiter.allow_request(client);
        let info = limiter.get_bucket_info(client);
        assert!(info.is_some());
        assert_eq!(info.unwrap().0, 1);
    }
}
