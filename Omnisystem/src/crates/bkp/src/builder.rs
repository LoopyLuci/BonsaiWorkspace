//! BKP package builder

use crate::error::{BkpError, BkpResult};
use crate::manifest::{BkpManifest, BaseModelInfo, KmodInfo, AdapterInfo};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

/// Builder for creating .bkp packages
///
/// Provides a fluent API for constructing BKP archives with:
/// - Base model (GGUF or other format)
/// - KMOD knowledge modules
/// - LoRA/QLoRA adapters
/// - Metadata and signatures
pub struct BkpBuilder {
    name: String,
    version: String,
    manifest: Option<BkpManifest>,
    files_to_add: Vec<(String, PathBuf)>, // (path_in_package, source_path)
    description: String,
    tags: Vec<String>,
}

impl BkpBuilder {
    /// Create a new BKP builder
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> BkpResult<Self> {
        Ok(Self {
            name: name.into(),
            version: version.into(),
            manifest: None,
            files_to_add: Vec::new(),
            description: String::new(),
            tags: Vec::new(),
        })
    }

    /// Add the base model file (typically GGUF)
    pub fn add_base_model(&mut self, path: impl AsRef<Path>) -> BkpResult<&mut Self> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(BkpError::NotFound(path.to_path_buf()));
        }

        let metadata = std::fs::metadata(path)?;
        let hash = compute_blake3(path)?;

        let base_model = BaseModelInfo {
            name: path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("model")
                .to_string(),
            architecture: "llama".to_string(), // Would parse from GGUF in production
            quantization: "q4_k_m".to_string(), // Would parse from GGUF
            size_bytes: metadata.len(),
            hash,
            path_in_package: "base_model/model.gguf".to_string(),
        };

        self.manifest = Some(BkpManifest::new(
            self.name.clone(),
            self.version.clone(),
            base_model,
        ));

        self.files_to_add
            .push(("base_model/model.gguf".to_string(), path.to_path_buf()));

        Ok(self)
    }

    /// Add a KMOD knowledge module
    pub fn add_kmod_module(
        &mut self,
        path: impl AsRef<Path>,
        name: impl Into<String>,
    ) -> BkpResult<&mut Self> {
        let path = path.as_ref();
        let name = name.into();

        if !path.exists() {
            return Err(BkpError::NotFound(path.to_path_buf()));
        }

        let metadata = std::fs::metadata(path)?;
        let hash = compute_blake3(path)?;

        let kmod_info = KmodInfo {
            name: name.clone(),
            domain: "knowledge".to_string(),
            version: "1.0.0".to_string(),
            entry_count: 0,
            size_bytes: metadata.len(),
            hash,
            path_in_package: format!("modules/{}.kmod", name),
        };

        if let Some(manifest) = &mut self.manifest {
            manifest.kmod_modules.push(kmod_info);
        }

        self.files_to_add
            .push((format!("modules/{}.kmod", name), path.to_path_buf()));

        Ok(self)
    }

    /// Add a LoRA/QLoRA adapter
    pub fn add_adapter(
        &mut self,
        path: impl AsRef<Path>,
        name: impl Into<String>,
        adapter_type: impl Into<String>,
    ) -> BkpResult<&mut Self> {
        let path = path.as_ref();
        let name = name.into();
        let adapter_type = adapter_type.into();

        if !path.exists() {
            return Err(BkpError::NotFound(path.to_path_buf()));
        }

        let metadata = std::fs::metadata(path)?;
        let hash = compute_blake3(path)?;

        let adapter_info = AdapterInfo {
            name: name.clone(),
            adapter_type,
            version: "1.0.0".to_string(),
            size_bytes: metadata.len(),
            hash,
            path_in_package: format!("adapters/{}.safetensors", name),
        };

        if let Some(manifest) = &mut self.manifest {
            manifest.adapters.push(adapter_info);
        }

        self.files_to_add
            .push((format!("adapters/{}.safetensors", name), path.to_path_buf()));

        Ok(self)
    }

    /// Set package description
    pub fn set_description(&mut self, description: impl Into<String>) -> &mut Self {
        self.description = description.into();
        if let Some(manifest) = &mut self.manifest {
            manifest.set_description(self.description.clone());
        }
        self
    }

    /// Add a tag to the package
    pub fn add_tag(&mut self, tag: impl Into<String>) -> &mut Self {
        self.tags.push(tag.into());
        if let Some(manifest) = &mut self.manifest {
            manifest.add_tag(self.tags.last().unwrap().clone());
        }
        self
    }

    /// Sign the manifest with an Ed25519 private key
    pub fn sign_manifest(&mut self, private_key_bytes: &[u8; 32]) -> BkpResult<&mut Self> {
        if self.manifest.is_none() {
            return Err(BkpError::Invalid(
                "Cannot sign manifest before adding base model".to_string(),
            ));
        }

        let manifest_json = self.manifest.as_ref().unwrap().to_json()?;

        // Sign with Ed25519
        let signing_key = ed25519_dalek::SigningKey::from_bytes(private_key_bytes);
        let signature = signing_key.sign(manifest_json.as_bytes());
        let public_key = signing_key.verifying_key();

        if let Some(manifest) = &mut self.manifest {
            manifest.signature = Some(signature.to_string());
            manifest.public_key = Some(public_key.to_string());
        }

        Ok(self)
    }

    /// Finalize and create the BKP package
    pub fn finalize(&mut self, output_path: impl AsRef<Path>) -> BkpResult<()> {
        let output_path = output_path.as_ref();

        if self.manifest.is_none() {
            return Err(BkpError::Invalid(
                "Cannot finalize without base model".to_string(),
            ));
        }

        // Create output directory
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create a temporary ZIP file
        let temp_zip = tempfile::NamedTempFile::new()?;
        let temp_path = temp_zip.path().to_path_buf();

        {
            let file = std::fs::File::create(&temp_path)?;
            let mut zip = zip::ZipWriter::new(file);

            // Write manifest
            let manifest_json = self
                .manifest
                .as_ref()
                .unwrap()
                .to_json()?;
            zip.start_file("manifest.json", zip::FileOptions::default())?;
            zip.write_all(manifest_json.as_bytes())?;

            // Write all files
            for (package_path, source_path) in &self.files_to_add {
                let data = std::fs::read(source_path)?;
                zip.start_file(package_path, zip::FileOptions::default())?;
                zip.write_all(&data)?;
            }

            zip.finish()?;
        }

        // Compress with zstd
        let zip_data = std::fs::read(&temp_path)?;
        let compressed = zstd::encode_all(&zip_data[..], 0)
            .map_err(|e| BkpError::zstd(format!("Compression failed: {}", e)))?;

        std::fs::write(output_path, &compressed)?;

        tracing::info!(
            "Created BKP package: {} ({} bytes)",
            output_path.display(),
            compressed.len()
        );

        Ok(())
    }
}

fn compute_blake3<P: AsRef<Path>>(path: P) -> BkpResult<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();

    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_builder_creation() {
        let builder = BkpBuilder::new("test", "1.0.0");
        assert!(builder.is_ok());
    }

    #[test]
    fn test_add_base_model() {
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(b"gguf").unwrap();
        temp_file.flush().unwrap();

        let mut builder = BkpBuilder::new("test", "1.0.0").unwrap();
        let result = builder.add_base_model(temp_file.path());

        assert!(result.is_ok());
        assert!(builder.manifest.is_some());
    }
}
