use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Metadata about a crystal image (compressed model artifact)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystalMetadata {
    pub format_version: String,
    pub compression: CompressionAlgorithm,
    pub checksum: String,
    pub size_uncompressed: u64,
    pub size_compressed: u64,
    pub created_at: String,
    pub components: HashMap<String, ComponentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    SquashFS,
    Zstd,
    Gzip,
    Uncompressed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub path: String,
    pub size: u64,
    pub checksum: String,
    pub description: String,
}

/// Represents a Crystal image - the compact, verified model artifact
pub struct CrystalImage {
    path: PathBuf,
    metadata: Option<CrystalMetadata>,
}

impl CrystalImage {
    /// Open a crystal image from disk
    pub fn open(path: &Path) -> Result<Self> {
        if !path.exists() {
            anyhow::bail!("Crystal image not found: {}", path.display());
        }

        let metadata_path = path.parent().unwrap().join("crystal.json");
        let metadata = if metadata_path.exists() {
            let content = std::fs::read_to_string(&metadata_path)?;
            Some(serde_json::from_str(&content)?)
        } else {
            None
        };

        Ok(Self {
            path: path.to_path_buf(),
            metadata,
        })
    }

    /// Get the crystal image metadata
    pub fn metadata(&self) -> Option<&CrystalMetadata> {
        self.metadata.as_ref()
    }

    /// Extract a file from the crystal image
    pub fn extract_file(&self, inner_path: &str) -> Result<Vec<u8>> {
        match self.metadata.as_ref().map(|m| &m.compression) {
            Some(CompressionAlgorithm::SquashFS) => {
                self.extract_from_squashfs(inner_path)
            }
            Some(CompressionAlgorithm::Zstd) => {
                self.extract_from_zstd(inner_path)
            }
            _ => {
                // Fallback: assume file exists next to image
                let full = self.path.parent().unwrap().join(inner_path);
                std::fs::read(full)
                    .map_err(|e| anyhow!("Failed to read file {}: {}", inner_path, e))
            }
        }
    }

    fn extract_from_squashfs(&self, inner_path: &str) -> Result<Vec<u8>> {
        // Placeholder: Real implementation would mount squashfs
        let search_dir = self.path.parent().unwrap();
        let candidate = search_dir.join(inner_path);
        if candidate.exists() {
            std::fs::read(&candidate)
                .map_err(|e| anyhow!("Failed to read squashfs file: {}", e))
        } else {
            anyhow::bail!("File not found in crystal image: {}", inner_path)
        }
    }

    fn extract_from_zstd(&self, inner_path: &str) -> Result<Vec<u8>> {
        // Placeholder: Real implementation would decompress
        let search_dir = self.path.parent().unwrap();
        let candidate = search_dir.join(inner_path);
        if candidate.exists() {
            let compressed = std::fs::read(&candidate)?;
            // Would use zstd::decode_all here
            Ok(compressed)
        } else {
            anyhow::bail!("File not found in zstd archive: {}", inner_path)
        }
    }

    /// Verify the crystal image integrity
    pub fn verify(&self) -> Result<bool> {
        match &self.metadata {
            Some(meta) => {
                let file_data = std::fs::read(&self.path)?;
                let calculated_hash = blake3::hash(&file_data).to_hex().to_string();
                Ok(calculated_hash == meta.checksum)
            }
            None => Ok(false),
        }
    }

    /// Get path to the crystal image
    pub fn path(&self) -> &Path {
        &self.path
    }
}
