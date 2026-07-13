use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum TransactionPhase {
    Prepare,
    Commit,
    Rollback,
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum SagaPhase {
    Pending,
    Executing,
    Compensating,
    Complete,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionParticipant {
    pub service_id: String,
    pub timestamp: DateTime<Utc>,
    pub prepared: bool,
    pub ready_to_commit: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributedTransaction {
    pub transaction_id: String,
    pub phase: TransactionPhase,
    pub participants: Vec<TransactionParticipant>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SagaStep {
    pub step_id: String,
    pub service_id: String,
    pub action: String,
    pub compensation: String,
    pub order: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SagaExecution {
    pub saga_id: String,
    pub steps: Vec<SagaStep>,
    pub phase: SagaPhase,
    pub completed_steps: Vec<String>,
    pub failed_step: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributedLock {
    pub lock_id: String,
    pub resource_id: String,
    pub owner: String,
    pub acquired_at: DateTime<Utc>,
    pub ttl_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub conflict_id: String,
    pub service_id: String,
    pub version_local: u32,
    pub version_remote: u32,
    pub resolution: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoordinationMetrics {
    pub total_transactions: u64,
    pub successful_commits: u64,
    pub failed_rollbacks: u64,
    pub avg_phase_latency_ms: f64,
    pub conflict_count: u64,
    pub lock_contention_percent: f64,
}
