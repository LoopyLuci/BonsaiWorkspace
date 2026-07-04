use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub entity_id: Uuid,
    pub name: String,
    pub entity_type: String,
    pub properties: Vec<(String, String)>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relationship {
    pub relationship_id: Uuid,
    pub source_entity: Uuid,
    pub target_entity: Uuid,
    pub relationship_type: String,
    pub properties: Vec<(String, String)>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Triple {
    pub triple_id: Uuid,
    pub subject: Uuid,
    pub predicate: String,
    pub object: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphQuery {
    pub query_id: Uuid,
    pub pattern: String,
    pub limit: u32,
    pub results_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathResult {
    pub path_id: Uuid,
    pub start_entity: Uuid,
    pub end_entity: Uuid,
    pub path_length: u32,
    pub path_entities: Vec<Uuid>,
}
