use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feature {
    pub feature_id: Uuid,
    pub name: String,
    pub feature_group: String,
    pub data_type: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureValue {
    pub value_id: Uuid,
    pub feature_id: Uuid,
    pub entity_id: String,
    pub value: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureGroup {
    pub group_id: Uuid,
    pub name: String,
    pub description: String,
    pub features: Vec<String>,
    pub owner: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureVersion {
    pub version_id: Uuid,
    pub feature_id: Uuid,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureSet {
    pub set_id: Uuid,
    pub name: String,
    pub features: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureMetadata {
    pub metadata_id: Uuid,
    pub feature_id: Uuid,
    pub statistics: Vec<(String, f32)>,
    pub data_quality_score: f32,
    pub last_updated: DateTime<Utc>,
}
