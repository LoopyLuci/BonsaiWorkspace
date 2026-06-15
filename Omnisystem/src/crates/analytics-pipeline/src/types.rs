use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataRecord {
    pub record_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub data: Vec<(String, String)>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pipeline {
    pub pipeline_id: Uuid,
    pub name: String,
    pub stages: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransformationRule {
    pub rule_id: Uuid,
    pub name: String,
    pub source_field: String,
    pub target_field: String,
    pub rule_type: RuleType,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum RuleType {
    Filter,
    Normalize,
    Aggregate,
    Enrich,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AggregationResult {
    pub result_id: Uuid,
    pub pipeline_id: Uuid,
    pub aggregations: Vec<(String, f32)>,
    pub record_count: u64,
    pub processed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub query_id: Uuid,
    pub query_text: String,
    pub rows: Vec<Vec<String>>,
    pub column_names: Vec<String>,
    pub execution_time_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataSchema {
    pub schema_id: Uuid,
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub version: String,
}
