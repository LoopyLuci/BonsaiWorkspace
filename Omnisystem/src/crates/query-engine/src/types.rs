use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Query {
    pub query_id: Uuid,
    pub sql: String,
    pub submitted_at: DateTime<Utc>,
    pub status: QueryStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum QueryStatus {
    Submitted,
    Parsing,
    Planning,
    Executing,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryPlan {
    pub plan_id: Uuid,
    pub query_id: Uuid,
    pub operations: Vec<String>,
    pub estimated_cost: f64,
    pub estimated_rows: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizedPlan {
    pub optimization_id: Uuid,
    pub plan_id: Uuid,
    pub original_cost: f64,
    pub optimized_cost: f64,
    pub optimization_rules: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub stats_id: Uuid,
    pub query_id: Uuid,
    pub rows_examined: u64,
    pub rows_returned: u64,
    pub execution_time_ms: u64,
    pub index_used: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexInfo {
    pub index_id: Uuid,
    pub index_name: String,
    pub table_name: String,
    pub column_names: Vec<String>,
    pub index_type: String,
    pub cardinality: u64,
}
