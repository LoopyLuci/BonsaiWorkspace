//! Data types
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// ID
    pub id: String,
    /// Created at
    pub created_at: DateTime<Utc>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
        }
    }
}

/// Agent input parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    /// Command to execute
    pub command: String,
    /// Command parameters
    pub parameters: HashMap<String, String>,
}

/// Agent execution output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Agent name
    pub agent_name: String,
    /// Execution status (success, error, etc.)
    pub status: String,
    /// Result data
    pub result: String,
}

/// Agent execution task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Task ID
    pub id: String,
    /// Agent name
    pub agent_name: String,
    /// Task status
    pub status: String,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
}

/// Agent metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Agent name
    pub agent_name: String,
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
    /// Average execution time (ms)
    pub avg_execution_time_ms: f64,
}
