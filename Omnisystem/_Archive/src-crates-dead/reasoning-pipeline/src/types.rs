use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReasoningQuery {
    pub query_id: Uuid,
    pub question: String,
    pub max_hops: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathStep {
    pub step_id: Uuid,
    pub hop_number: u32,
    pub entity: String,
    pub relation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReasoningChain {
    pub chain_id: Uuid,
    pub query_id: Uuid,
    pub steps: Vec<Uuid>,
    pub conclusion: String,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiHopResult {
    pub result_id: Uuid,
    pub query_id: Uuid,
    pub paths: Vec<Vec<String>>,
    pub total_hops: u32,
    pub found: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReasoningExplanation {
    pub explanation_id: Uuid,
    pub result_id: Uuid,
    pub reasoning_steps: Vec<String>,
    pub evidence: Vec<String>,
}
