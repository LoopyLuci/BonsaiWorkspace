use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub bkp_version: u32,
    pub base_model: BaseModelInfo,
    pub adapters: Vec<AdapterInfo>,
    pub knowledge_modules: Vec<KnowledgeModuleRef>,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseModelInfo {
    pub name: String,
    pub arch: String,
    pub quant: String,
    pub blake3: String,
    pub size_bytes: u64,
    pub path_in_package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub name: String,
    pub adapter_type: String,
    pub blake3: String,
    pub size_bytes: u64,
    pub path_in_package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeModuleRef {
    pub name: String,
    pub domain: String,
    pub version: String,
    pub entry_count: usize,
    pub path_in_package: String,
}

impl PackageManifest {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        base_model: BaseModelInfo,
    ) -> Self {
        PackageManifest {
            id: Uuid::new_v4(),
            name: name.into(),
            version: version.into(),
            created_at: Utc::now(),
            bkp_version: 1,
            base_model,
            adapters: Vec::new(),
            knowledge_modules: Vec::new(),
            description: String::new(),
            tags: Vec::new(),
        }
    }
}
