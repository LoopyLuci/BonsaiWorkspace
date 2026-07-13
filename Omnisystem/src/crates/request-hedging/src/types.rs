use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HedgeRequest {
    pub request_id: String,
    pub service_id: String,
    pub original_deadline: DateTime<Utc>,
    pub hedge_delay_ms: u64,
    pub max_hedges: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum HedgeOutcome {
    FirstSuccess,
    HedgeSuccess,
    AllFailed,
    TimedOut,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HedgeResult {
    pub request_id: String,
    pub outcome: HedgeOutcome,
    pub winning_attempt: u32,
    pub total_attempts: u32,
    pub total_latency_ms: u64,
    pub hedge_latencies: Vec<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HedgingMetrics {
    pub service_id: String,
    pub total_requests: u64,
    pub hedged_requests: u64,
    pub hedge_success_count: u64,
    pub first_attempt_success_rate: f64,
    pub avg_hedge_latency_ms: f64,
    pub p99_hedge_latency_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum ConsensusType {
    Majority,
    Unanimous,
    Quorum(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceVote {
    pub service_id: String,
    pub timestamp: DateTime<Utc>,
    pub decision: bool,
    pub confidence: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusDecision {
    pub decision_id: String,
    pub consensus_type: ConsensusType,
    pub votes: Vec<ServiceVote>,
    pub final_decision: bool,
    pub confidence_level: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub plan_id: String,
    pub service_order: Vec<String>,
    pub parallel_stages: Vec<Vec<String>>,
    pub dependencies: Vec<(String, String)>,
    pub estimated_duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrchestrationStatus {
    pub plan_id: String,
    pub completed_services: Vec<String>,
    pub failed_services: Vec<String>,
    pub in_progress_services: Vec<String>,
    pub overall_progress_percent: u32,
}
