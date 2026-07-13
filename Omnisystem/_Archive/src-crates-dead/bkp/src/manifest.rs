//! BKP manifest — metadata for packages

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Complete manifest for a BKP package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BkpManifest {
    /// Package identifier
    pub id: Uuid,

    /// Package name
    pub name: String,

    /// Package version (semantic versioning)
    pub version: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,

    /// BKP format version
    pub format_version: u32,

    /// Base model information
    pub base_model: BaseModelInfo,

    /// Description of package
    pub description: String,

    /// Tags/keywords
    pub tags: Vec<String>,

    /// KMOD modules in this package
    pub kmod_modules: Vec<KmodInfo>,

    /// Adapters in this package
    pub adapters: Vec<AdapterInfo>,

    /// File checksums for verification
    pub file_hashes: HashMap<String, FileHash>,

    /// Ed25519 signature (if signed)
    pub signature: Option<String>,

    /// Public key used for signature
    pub public_key: Option<String>,
}

/// Information about the base model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseModelInfo {
    pub name: String,
    pub architecture: String,
    pub quantization: String,
    pub size_bytes: u64,
    pub hash: String,
    pub path_in_package: String,
}

/// Information about a KMOD knowledge module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KmodInfo {
    pub name: String,
    pub domain: String,
    pub version: String,
    pub entry_count: usize,
    pub size_bytes: u64,
    pub hash: String,
    pub path_in_package: String,
}

/// Information about an adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub name: String,
    pub adapter_type: String,
    pub version: String,
    pub size_bytes: u64,
    pub hash: String,
    pub path_in_package: String,
}

/// File hash entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHash {
    pub algorithm: String, // "blake3" or "sha256"
    pub digest: String,
}

impl BkpManifest {
    /// Create a new manifest
    pub fn new(name: impl Into<String>, version: impl Into<String>, base_model: BaseModelInfo) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            version: version.into(),
            created_at: now,
            modified_at: now,
            format_version: 1,
            base_model,
            description: String::new(),
            tags: Vec::new(),
            kmod_modules: Vec::new(),
            adapters: Vec::new(),
            file_hashes: HashMap::new(),
            signature: None,
            public_key: None,
        }
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// Set description
    pub fn set_description(&mut self, desc: impl Into<String>) {
        self.description = desc.into();
    }

    /// Add a file hash entry
    pub fn add_file_hash(&mut self, path: impl Into<String>, algorithm: impl Into<String>, digest: impl Into<String>) {
        self.file_hashes.insert(
            path.into(),
            FileHash {
                algorithm: algorithm.into(),
                digest: digest.into(),
            },
        );
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_creation() {
        let base_model = BaseModelInfo {
            name: "test-model".to_string(),
            architecture: "llama".to_string(),
            quantization: "q4_k_m".to_string(),
            size_bytes: 1_000_000,
            hash: "abc123".to_string(),
            path_in_package: "base_model/model.gguf".to_string(),
        };

        let manifest = BkpManifest::new("test-pkg", "1.0.0", base_model);
        assert_eq!(manifest.name, "test-pkg");
        assert_eq!(manifest.version, "1.0.0");
    }

    #[test]
    fn test_manifest_serialization() {
        let base_model = BaseModelInfo {
            name: "test-model".to_string(),
            architecture: "llama".to_string(),
            quantization: "q4_k_m".to_string(),
            size_bytes: 1_000_000,
            hash: "abc123".to_string(),
            path_in_package: "base_model/model.gguf".to_string(),
        };

        let manifest = BkpManifest::new("test-pkg", "1.0.0", base_model);
        let json = manifest.to_json().unwrap();
        let restored = BkpManifest::from_json(&json).unwrap();

        assert_eq!(manifest.name, restored.name);
        assert_eq!(manifest.version, restored.version);
    }
}
