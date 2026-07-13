use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dataset {
    pub dataset_id: Uuid,
    pub name: String,
    pub owner: String,
    pub location: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub metadata_id: Uuid,
    pub dataset_id: Uuid,
    pub record_count: u64,
    pub size_bytes: u64,
    pub format: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasetTag {
    pub tag_id: Uuid,
    pub dataset_id: Uuid,
    pub tag: String,
    pub category: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatasetOwnership {
    pub ownership_id: Uuid,
    pub dataset_id: Uuid,
    pub owner_name: String,
    pub owner_email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub result_id: Uuid,
    pub dataset_id: Uuid,
    pub match_score: f32,
    pub relevant_fields: Vec<String>,
}
