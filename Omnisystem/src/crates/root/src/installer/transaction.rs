use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use crate::manifest::Component;

pub struct Transaction {
    staging_dir: TempDir,
    commit_plan: Vec<(PathBuf, PathBuf)>,
    rollback_snapshot: PathBuf,
}

impl Transaction {
    pub fn new() -> Result<Self> {
        let staging_dir = TempDir::new().context("failed to create transaction staging directory")?;
        let rollback_snapshot = crate::utils::rollback_dir()?.join(Utc::now().timestamp().to_string());
        fs::create_dir_all(&rollback_snapshot)?;
        Ok(Self {
            staging_dir,
            commit_plan: Vec::new(),
            rollback_snapshot,
        })
    }

    pub async fn download_and_stage(
        &mut self,
        component: &Component,
        dest_root: &Path,
    ) -> Result<()> {
        let dest_dir = dest_root.join(&component.id);
        let zip_path = self
            .staging_dir
            .path()
            .join(format!("{}.zip", component.id));

        download_file(&component.download_url, &zip_path).await?;

        let hash = compute_hash(&zip_path)?;
        if hash.to_lowercase() != component.hash.to_lowercase() {
            return Err(anyhow!(
                "hash mismatch for {}: expected {}, got {}",
                component.name,
                component.hash,
                hash
            ));
        }

        let extract_dir = self.staging_dir.path().join(&component.id);
        extract_zip(&zip_path, &extract_dir)?;

        self.commit_plan.push((extract_dir, dest_dir.clone()));

        if dest_dir.exists() {
            let rollback_copy = self.rollback_snapshot.join(&component.id);
            if rollback_copy.exists() {
                fs::remove_dir_all(&rollback_copy)?;
            }
            fs::rename(&dest_dir, &rollback_copy)
                .with_context(|| format!("failed to snapshot {}", component.id))?;
        }

        Ok(())
    }

    pub fn commit(self) -> Result<()> {
        for (src, dst) in &self.commit_plan {
            if dst.exists() {
                fs::remove_dir_all(dst).with_context(|| {
                    format!("failed to remove existing destination {}", dst.display())
                })?;
            }
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(src, dst).with_context(|| {
                format!("failed to commit {} -> {}", src.display(), dst.display())
            })?;
        }
        Ok(())
    }

    pub fn rollback(&self, dest_root: &Path) -> Result<()> {
        if !self.rollback_snapshot.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.rollback_snapshot)? {
            let entry = entry?;
            let component_id = entry.file_name();
            let original_path = dest_root.join(component_id);
            if original_path.exists() {
                fs::remove_dir_all(&original_path)?;
            }
            fs::rename(entry.path(), &original_path).with_context(|| {
                format!("failed to restore rollback snapshot into {}", original_path.display())
            })?;
        }
        Ok(())
    }
}

pub async fn download_file(url: &str, dest: &Path) -> Result<()> {
    let bytes = reqwest::get(url)
        .await
        .with_context(|| format!("failed to GET {url}"))?
        .error_for_status()
        .with_context(|| format!("download failed with non-success status for {url}"))?
        .bytes()
        .await
        .with_context(|| format!("failed reading response body for {url}"))?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    tokio::fs::write(dest, &bytes)
        .await
        .with_context(|| format!("failed writing download to {}", dest.display()))
}

pub fn compute_hash(path: &Path) -> Result<String> {
    let file = fs::File::open(path)
        .with_context(|| format!("failed to open file for hashing: {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];

    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    let file = fs::File::open(zip_path)
        .with_context(|| format!("failed to open zip archive {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("failed to parse zip archive")?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| anyhow!("archive contains invalid path"))?
            .to_path_buf();
        let out_path = dest.join(enclosed);

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out_file = fs::File::create(&out_path)?;
        io::copy(&mut entry, &mut out_file)?;
    }

    Ok(())
}
