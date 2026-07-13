use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataLineage {
    pub lineage_id: Uuid,
    pub source_dataset_id: Uuid,
    pub target_dataset_id: Uuid,
    pub transformation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub dep_id: Uuid,
    pub source_dataset: String,
    pub target_dataset: String,
    pub dep_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provenance {
    pub prov_id: Uuid,
    pub dataset_id: Uuid,
    pub origin: String,
    pub transformations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transformation {
    pub transform_id: Uuid,
    pub dataset_id: Uuid,
    pub transform_type: String,
    pub script: String,
}
