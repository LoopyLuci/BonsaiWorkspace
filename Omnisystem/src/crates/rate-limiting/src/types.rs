use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum RateLimitAlgorithm {
    TokenBucket,
    LeakyBucket,
    SlidingWindow,
    FixedWindow,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy, PartialOrd, Ord)]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenBucketConfig {
    pub capacity: u64,
    pub refill_rate: u64,
    pub refill_interval_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenBucket {
    pub bucket_id: String,
    pub tokens: f64,
    pub capacity: u64,
    pub refill_rate: u64,
    pub refill_interval_ms: u64,
    pub last_refill: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitWindow {
    pub window_id: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub limit: u64,
    pub current_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitPolicy {
    pub policy_id: String,
    pub algorithm: RateLimitAlgorithm,
    pub requests_per_second: u64,
    pub burst_capacity: u64,
    pub window_size_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub tokens_remaining: f64,
    pub retry_after_ms: Option<u64>,
    pub priority_level: RequestPriority,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributedQuota {
    pub quota_id: String,
    pub service_id: String,
    pub total_allowance: u64,
    pub used_allowance: u64,
    pub reset_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitMetrics {
    pub total_requests: u64,
    pub allowed_requests: u64,
    pub rejected_requests: u64,
    pub avg_wait_time_ms: f64,
    pub rejection_rate: f64,
    pub current_qps: f64,
}
