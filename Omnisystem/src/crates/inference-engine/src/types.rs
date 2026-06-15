use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub rule_id: Uuid,
    pub name: String,
    pub conditions: Vec<String>,
    pub conclusion: String,
    pub priority: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fact {
    pub fact_id: Uuid,
    pub predicate: String,
    pub value: String,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceOutcome {
    pub result_id: Uuid,
    pub rule_id: Uuid,
    pub derived_fact: String,
    pub confidence: f32,
    pub inference_chain: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub kb_id: Uuid,
    pub rule_count: u32,
    pub fact_count: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeductiveStep {
    pub step_id: Uuid,
    pub rule_applied: Uuid,
    pub input_facts: Vec<Uuid>,
    pub output_fact: String,
}
