use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{KdbError, Result};
use hnsw::{Distance, HnswIndex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub domain: String,
    pub description: String,
    pub dim: usize,
    pub entry_count: usize,
    pub distance: Distance,
    pub created_at: DateTime<Utc>,
    pub blake3_index: String,
    pub blake3_values: String,
}

#[derive(Debug)]
pub struct LoadedModule {
    pub manifest: ModuleManifest,
    pub index: HnswIndex,
    pub values: Vec<String>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub version: String,
    pub entry_count: usize,
    pub path: PathBuf,
}

impl LoadedModule {
    pub fn load(dir: &Path) -> Result<Self> {
        let manifest_path = dir.join("manifest.json");
        let manifest: ModuleManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;

        let index_path = dir.join("index.hnsw");
        let index = HnswIndex::load(&index_path)?;

        let values_path = dir.join("values.txt");
        let values_raw = if values_path.exists() {
            fs::read_to_string(&values_path)?
        } else {
            let zst_path = dir.join("values.txt.zst");
            if zst_path.exists() {
                let compressed = fs::read(&zst_path)?;
                let decoded = zstd::decode_all(compressed.as_slice()).map_err(KdbError::Io)?;
                String::from_utf8(decoded)
                    .map_err(|e| KdbError::Invalid(format!("values utf8: {e}")))?
            } else {
                return Err(KdbError::Invalid("no values.txt or values.txt.zst".into()));
            }
        };

        let values: Vec<String> = values_raw.lines().map(str::to_owned).collect();

        if values.len() != manifest.entry_count {
            return Err(KdbError::Invalid(format!(
                "values count {} != manifest entry_count {}",
                values.len(),
                manifest.entry_count
            )));
        }

        Ok(LoadedModule {
            manifest,
            index,
            values,
            path: dir.to_path_buf(),
        })
    }

    pub fn info(&self) -> ModuleInfo {
        ModuleInfo {
            id: self.manifest.id,
            name: self.manifest.name.clone(),
            domain: self.manifest.domain.clone(),
            version: self.manifest.version.clone(),
            entry_count: self.manifest.entry_count,
            path: self.path.clone(),
        }
    }
}
