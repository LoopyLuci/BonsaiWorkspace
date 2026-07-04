//! Runtime downloader with CAS integration

use crate::cas::{ContentAddressedStore, ContentHash};
use crate::runtime::manifest::RuntimeEntry;
use anyhow::{anyhow, Result};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct RuntimeDownloader {
    cas: ContentAddressedStore,
    cache_dir: PathBuf,
}

impl RuntimeDownloader {
    pub async fn new(cas: ContentAddressedStore, cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir).await?;
        Ok(Self { cas, cache_dir })
    }

    pub async fn download(&self, entry: &RuntimeEntry) -> Result<PathBuf> {
        let hash_str = entry.hash_without_algo()?;
        let hash = ContentHash(hash_str);
        let cas_path = self.cas.get_path(&hash);

        if cas_path.exists() {
            return Ok(cas_path);
        }

        tracing::info!(
            "Downloading {} {} from {}",
            entry.name,
            entry.version,
            entry.url
        );

        let resp = reqwest::get(&entry.url)
            .await
            .map_err(|e| anyhow!("Failed to fetch {}: {}", entry.url, e))?;

        let data = resp
            .bytes()
            .await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;

        if !entry.verify_hash(&data)? {
            return Err(anyhow!("Hash verification failed for {}", entry.full_id()));
        }

        tracing::info!("Hash verified, storing in CAS...");

        let temp_path = cas_path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(&data).await?;
        file.sync_all().await?;
        fs::rename(&temp_path, &cas_path).await?;

        tracing::info!("Runtime {} stored at {:?}", entry.full_id(), cas_path);

        Ok(cas_path)
    }

    pub async fn decompress_tar_xz(archive_path: &PathBuf, out_dir: &PathBuf) -> Result<()> {
        if out_dir.exists() {
            return Ok(());
        }

        tracing::info!("Decompressing {:?} to {:?}", archive_path, out_dir);

        fs::create_dir_all(&out_dir).await?;

        let archive = std::fs::File::open(archive_path)?;
        let xz_decoder = xz2::read::XzDecoder::new(archive);
        let mut archive = tar::Archive::new(xz_decoder);

        // Blocking task to avoid async issues with tar
        let out_dir_clone = out_dir.clone();
        tokio::task::spawn_blocking(move || archive.unpack(&out_dir_clone))
            .await??;

        tracing::info!("Decompression complete");

        Ok(())
    }

    pub async fn prepare_runtime(&self, entry: &RuntimeEntry) -> Result<PathBuf> {
        let archive_path = self.download(entry).await?;

        if entry.compressed {
            let extracted_dir = self.cache_dir.join(entry.full_id());
            Self::decompress_tar_xz(&archive_path, &extracted_dir).await?;
            Ok(extracted_dir)
        } else {
            Ok(archive_path)
        }
    }

    pub async fn is_cached(&self, entry: &RuntimeEntry) -> Result<bool> {
        let hash_str = entry.hash_without_algo()?;
        let hash = ContentHash(hash_str);
        Ok(self.cas.has(&hash).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_verification() {
        let data = b"test data";
        let hash = blake3::hash(data).to_hex().to_string();

        let entry = RuntimeEntry {
            name: "test".to_string(),
            version: "1.0".to_string(),
            platform: "x86_64".to_string(),
            url: "https://example.com/test".to_string(),
            hash: format!("blake3:{}", hash),
            signature: String::new(),
            compressed: false,
            build_script: None,
        };

        assert!(entry.verify_hash(data).unwrap());
    }
}
