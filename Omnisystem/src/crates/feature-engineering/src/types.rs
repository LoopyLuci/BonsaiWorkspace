use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feature {
    pub feature_id: Uuid,
    pub feature_name: String,
    pub entity_id: String,
    pub value: f64,
    pub created_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureDefinition {
    pub definition_id: Uuid,
    pub feature_name: String,
    pub description: String,
    pub dtype: FeatureDataType,
    pub computation_logic: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum FeatureDataType {
    Numerical,
    Categorical,
    Boolean,
    Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureVersion {
    pub version_id: Uuid,
    pub feature_name: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub status: VersionStatus,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum VersionStatus {
    Draft,
    Active,
    Deprecated,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeatureStoreEntry {
    pub store_id: Uuid,
    pub entity_id: String,
    pub entity_type: String,
    pub features: Vec<(String, f64)>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
