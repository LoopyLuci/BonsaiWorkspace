use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum FailureType {
    Timeout,
    Exception,
    HttpError,
    ConnectionRefused,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub breaker_id: String,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_ms: u64,
    pub half_open_max_calls: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitBreakerMetrics {
    pub breaker_id: String,
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub rejected_calls: u64,
    pub failure_rate: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateTransition {
    pub from_state: CircuitState,
    pub to_state: CircuitState,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitBreakerStatus {
    pub breaker_id: String,
    pub current_state: CircuitState,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_failure_time: Option<DateTime<Utc>>,
    pub opened_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HalfOpenMetrics {
    pub half_open_calls: u32,
    pub half_open_successes: u32,
    pub half_open_failures: u32,
    pub recovery_attempts: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FallbackPolicy {
    pub policy_id: String,
    pub fallback_enabled: bool,
    pub fallback_timeout_ms: u64,
    pub max_fallback_calls: u32,
    pub fallback_success_rate: f64,
}
