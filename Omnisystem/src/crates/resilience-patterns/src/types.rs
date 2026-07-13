use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    pub initial_timeout_ms: u64,
    pub max_timeout_ms: u64,
    pub adaptive: bool,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            initial_timeout_ms: 5000,
            max_timeout_ms: 30000,
            adaptive: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter_enabled: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10000,
            backoff_multiplier: 2.0,
            jitter_enabled: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BulkheadPolicy {
    pub max_concurrent_calls: usize,
    pub queue_capacity: usize,
    pub timeout_duration_ms: u64,
}

impl Default for BulkheadPolicy {
    fn default() -> Self {
        Self {
            max_concurrent_calls: 100,
            queue_capacity: 1000,
            timeout_duration_ms: 60000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HedgingPolicy {
    pub enabled: bool,
    pub hedge_delay_ms: u64,
    pub max_hedges: u32,
}

impl Default for HedgingPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            hedge_delay_ms: 100,
            max_hedges: 2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IsolationPolicy {
    pub thread_pool_size: usize,
    pub isolation_enabled: bool,
    pub failure_threshold: f64,
}

impl Default for IsolationPolicy {
    fn default() -> Self {
        Self {
            thread_pool_size: 10,
            isolation_enabled: true,
            failure_threshold: 0.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptiveTimeoutConfig {
    pub service_id: String,
    pub current_timeout_ms: u64,
    pub p99_latency_ms: u64,
    pub success_count: u64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestMetadata {
    pub request_id: String,
    pub attempt_count: u32,
    pub start_time: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
    pub context: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BulkheadMetrics {
    pub service_id: String,
    pub active_calls: usize,
    pub queued_calls: usize,
    pub total_calls: u64,
    pub rejected_calls: u64,
    pub timeout_calls: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResilienceMetrics {
    pub service_id: String,
    pub retry_count: u64,
    pub success_after_retry: u64,
    pub failure_after_retry: u64,
    pub hedge_count: u64,
    pub hedge_success_count: u64,
    pub avg_timeout_used_ms: f64,
}
