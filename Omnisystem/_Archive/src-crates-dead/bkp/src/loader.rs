//! BKP package loader — extract and verify packages

use crate::error::{BkpError, BkpResult};
use crate::manifest::BkpManifest;
use std::path::{Path, PathBuf};
use std::io::Read;

/// Loader for extracting and verifying .bkp packages
pub struct BkpLoader {
    path: PathBuf,
    manifest: Option<BkpManifest>,
}

impl BkpLoader {
    /// Open a BKP package
    pub fn new(path: impl AsRef<Path>) -> BkpResult<Self> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            return Err(BkpError::NotFound(path));
        }

        let loader = Self {
            path,
            manifest: None,
        };

        Ok(loader)
    }

    /// Load and return the manifest
    pub fn manifest(&mut self) -> BkpResult<&BkpManifest> {
        if self.manifest.is_none() {
            self.manifest = Some(self.load_manifest()?);
        }

        Ok(self.manifest.as_ref().unwrap())
    }

    /// Verify the package signature with a public key
    pub fn verify_signature(&self, public_key_hex: &str) -> BkpResult<bool> {
        let manifest_json = self.load_manifest()?.to_json()?;

        if let Some(manifest) = self.manifest.as_ref() {
            if let Some(sig_str) = &manifest.signature {
                // Verify Ed25519 signature
                let public_key_bytes = hex::decode(public_key_hex)
                    .map_err(|e| BkpError::SignatureVerification(e.to_string()))?;

                let public_key = ed25519_dalek::VerifyingKey::from_bytes(
                    public_key_bytes
                        .as_slice()
                        .try_into()
                        .map_err(|_| BkpError::SignatureVerification(
                            "Invalid public key length".to_string(),
                        ))?,
                )
                .map_err(|e| BkpError::SignatureVerification(e.to_string()))?;

                let signature_bytes = hex::decode(sig_str)
                    .map_err(|e| BkpError::SignatureVerification(e.to_string()))?;

                let signature = ed25519_dalek::Signature::from_bytes(
                    signature_bytes
                        .as_slice()
                        .try_into()
                        .map_err(|_| BkpError::SignatureVerification(
                            "Invalid signature length".to_string(),
                        ))?,
                );

                return match public_key.verify(manifest_json.as_bytes(), &signature) {
                    Ok(()) => Ok(true),
                    Err(_) => Ok(false),
                };
            }
        }

        Ok(false)
    }

    /// Extract the entire package to a directory
    pub fn extract_to(&self, dest_dir: impl AsRef<Path>) -> BkpResult<()> {
        let dest_dir = dest_dir.as_ref();
        std::fs::create_dir_all(dest_dir)?;

        let zip_data = self.decompress_bkp()?;
        let cursor = std::io::Cursor::new(zip_data);
        let mut archive = zip::ZipArchive::new(cursor)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = dest_dir.join(file.name());

            if file.is_dir() {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        tracing::info!(
            "Extracted BKP to {}",
            dest_dir.display()
        );

        Ok(())
    }

    /// Extract a specific file from the package
    pub fn extract_file(&self, file_path: &str) -> BkpResult<PathBuf> {
        let zip_data = self.decompress_bkp()?;
        let cursor = std::io::Cursor::new(zip_data);
        let mut archive = zip::ZipArchive::new(cursor)?;

        // Find the file
        let mut found = false;
        let mut file_data = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name() == file_path {
                file.read_to_end(&mut file_data)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(BkpError::NotFound(PathBuf::from(file_path)));
        }

        // Write to temporary file
        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path().to_path_buf();

        std::fs::write(&temp_path, &file_data)?;

        Ok(temp_path)
    }

    /// Get the base model file (extracts to temp location)
    pub fn extract_base_model(&self) -> BkpResult<PathBuf> {
        let manifest = self.load_manifest()?;
        self.extract_file(&manifest.base_model.path_in_package)
    }

    /// Get all KMOD modules
    pub fn list_kmod_modules(&self) -> BkpResult<Vec<String>> {
        let manifest = self.load_manifest()?;
        Ok(manifest
            .kmod_modules
            .iter()
            .map(|m| m.name.clone())
            .collect())
    }

    /// Get all adapters
    pub fn list_adapters(&self) -> BkpResult<Vec<String>> {
        let manifest = self.load_manifest()?;
        Ok(manifest
            .adapters
            .iter()
            .map(|a| a.name.clone())
            .collect())
    }

    /// Extract a KMOD module by name
    pub fn extract_kmod_module(&self, name: &str) -> BkpResult<PathBuf> {
        let manifest = self.load_manifest()?;
        let kmod = manifest
            .kmod_modules
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| BkpError::NotFound(PathBuf::from(format!("kmod:{}", name))))?;

        self.extract_file(&kmod.path_in_package)
    }

    /// Extract an adapter by name
    pub fn extract_adapter(&self, name: &str) -> BkpResult<PathBuf> {
        let manifest = self.load_manifest()?;
        let adapter = manifest
            .adapters
            .iter()
            .find(|a| a.name == name)
            .ok_or_else(|| BkpError::NotFound(PathBuf::from(format!("adapter:{}", name))))?;

        self.extract_file(&adapter.path_in_package)
    }

    fn load_manifest(&self) -> BkpResult<BkpManifest> {
        let zip_data = self.decompress_bkp()?;
        let cursor = std::io::Cursor::new(zip_data);
        let mut archive = zip::ZipArchive::new(cursor)?;

        // Find manifest.json
        let mut manifest_data = Vec::new();
        let mut found = false;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name() == "manifest.json" {
                file.read_to_end(&mut manifest_data)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(BkpError::InvalidManifest("manifest.json not found".to_string()));
        }

        let manifest_json = String::from_utf8(manifest_data)
            .map_err(|e| BkpError::InvalidManifest(e.to_string()))?;

        BkpManifest::from_json(&manifest_json)
            .map_err(|e| BkpError::InvalidManifest(e.to_string()))
    }

    fn decompress_bkp(&self) -> BkpResult<Vec<u8>> {
        let compressed_data = std::fs::read(&self.path)?;

        zstd::decode_all(&compressed_data[..])
            .map_err(|e| BkpError::zstd(format!("Decompression failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_not_found() {
        let result = BkpLoader::new("/nonexistent/path.bkp");
        assert!(result.is_err());
    }
}
