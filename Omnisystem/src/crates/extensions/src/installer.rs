//! Extension Installer — fetch from GitHub, validate manifest, scan, install.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{
    manifest::ExtensionManifest,
    registry::{ExtensionRegistry, InstalledExtension},
    scanner::SecurityScanner,
    ExtensionError,
};

/// Progress events emitted during installation (for UI streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "step")]
pub enum InstallProgress {
    Fetching { url: String },
    Validating,
    Scanning { files_done: usize },
    Installing,
    Completed { extension_id: String },
    Failed { reason: String },
}

/// Result of a completed installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub extension_id: String,
    pub version: String,
    pub install_path: String,
    pub verdict_label: String,
    pub risk_score: u8,
    pub files_scanned: usize,
    pub finding_count: usize,
}

/// The installer coordinates fetching, scanning, and registering.
pub struct Installer {
    scanner: SecurityScanner,
    registry: ExtensionRegistry,
}

impl Installer {
    pub fn new(registry: ExtensionRegistry) -> Self {
        Self { scanner: SecurityScanner::new(), registry }
    }

    /// Install from a local directory (already cloned/extracted).
    /// Used for local-dev installs and after GitHub clone.
    pub async fn install_from_dir(
        &self,
        dir: &Path,
        source_url: Option<&str>,
    ) -> Result<InstallResult, ExtensionError> {
        // 1. Locate and parse manifest
        let manifest = self.load_manifest(dir)?;
        manifest.validate().map_err(ExtensionError::InvalidManifest)?;

        info!(ext = %manifest.extension_id, version = %manifest.version, "[installer] installing");

        // 2. Run security scan
        let report = self
            .scanner
            .scan(dir, &manifest.extension_id, &manifest.version)
            .await;

        let verdict_label = report.verdict.to_string();
        let risk_score = report.risk_score;
        let files_scanned = report.files_scanned;
        let finding_count = report.findings.len();

        // 3. Determine install path
        let install_path = self
            .registry
            .base_dir
            .join(&manifest.extension_id)
            .join(&manifest.version);
        tokio::fs::create_dir_all(&install_path)
            .await
            .map_err(|e| ExtensionError::Io(e.to_string()))?;

        // 4. Copy files to install path
        self.copy_extension_files(dir, &install_path).await?;

        // 5. Write report
        let report_path = install_path.join("security_report.json");
        let report_json = serde_json::to_string_pretty(&report)
            .map_err(|e| ExtensionError::Io(e.to_string()))?;
        tokio::fs::write(&report_path, report_json)
            .await
            .map_err(|e| ExtensionError::Io(e.to_string()))?;

        // 6. Register
        let mut ext = InstalledExtension::new(manifest.clone(), install_path.clone());
        if let Some(url) = source_url {
            ext.source_path = Some(PathBuf::from(url));
        }
        ext.last_security_report = Some(report);
        self.registry.register(ext);

        info!(ext = %manifest.extension_id, "[installer] install complete");

        Ok(InstallResult {
            extension_id: manifest.extension_id,
            version: manifest.version,
            install_path: install_path.to_string_lossy().to_string(),
            verdict_label,
            risk_score,
            files_scanned,
            finding_count,
        })
    }

    /// Install from a GitHub URL by cloning to a temp directory.
    pub async fn install_from_github(
        &self,
        github_url: &str,
    ) -> Result<InstallResult, ExtensionError> {
        info!(url = github_url, "[installer] fetching from GitHub");

        // Derive a temp directory name from the URL hash
        let url_hash = {
            let mut h = blake3::Hasher::new();
            h.update(github_url.as_bytes());
            hex::encode(&h.finalize().as_bytes()[..8])
        };
        let tmp_dir = self
            .registry
            .base_dir
            .join("_tmp")
            .join(&url_hash);

        // Remove stale temp dir if present
        if tmp_dir.exists() {
            tokio::fs::remove_dir_all(&tmp_dir)
                .await
                .map_err(|e| ExtensionError::Io(e.to_string()))?;
        }
        tokio::fs::create_dir_all(&tmp_dir)
            .await
            .map_err(|e| ExtensionError::Io(e.to_string()))?;

        // Download the zip archive from GitHub (default branch).
        // Converts https://github.com/user/repo → https://github.com/user/repo/archive/refs/heads/main.zip
        let archive_url = if github_url.ends_with(".git") {
            format!(
                "{}/archive/refs/heads/main.zip",
                github_url.trim_end_matches(".git")
            )
        } else {
            format!("{}/archive/refs/heads/main.zip", github_url.trim_end_matches('/'))
        };

        let response = reqwest::get(&archive_url)
            .await
            .map_err(|e| ExtensionError::Fetch(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ExtensionError::Fetch(format!(
                "HTTP {} from {}",
                response.status(),
                archive_url
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ExtensionError::Fetch(e.to_string()))?;

        // Extract zip
        let archive_path = tmp_dir.join("archive.zip");
        tokio::fs::write(&archive_path, &bytes)
            .await
            .map_err(|e| ExtensionError::Io(e.to_string()))?;

        let extract_dir = tmp_dir.join("extracted");
        tokio::fs::create_dir_all(&extract_dir)
            .await
            .map_err(|e| ExtensionError::Io(e.to_string()))?;

        // Unzip using the zip crate via spawn_blocking (CPU-bound)
        let archive_path_clone = archive_path.clone();
        let extract_dir_clone = extract_dir.clone();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&archive_path_clone)?;
            let mut archive = zip::ZipArchive::new(file)?;
            archive.extract(&extract_dir_clone)?;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        })
        .await
        .map_err(|e| ExtensionError::Io(e.to_string()))?
        .map_err(|e| ExtensionError::Io(e.to_string()))?;

        // GitHub archives have a single top-level directory: `{repo}-main/`
        let mut repo_dir = extract_dir.clone();
        let mut read_dir = tokio::fs::read_dir(&extract_dir)
            .await
            .map_err(|e| ExtensionError::Io(e.to_string()))?;
        if let Ok(Some(entry)) = read_dir.next_entry().await {
            repo_dir = entry.path();
        }

        let result = self.install_from_dir(&repo_dir, Some(github_url)).await?;

        // Clean up temp files
        let _ = tokio::fs::remove_dir_all(&tmp_dir).await;

        Ok(result)
    }

    fn load_manifest(&self, dir: &Path) -> Result<ExtensionManifest, ExtensionError> {
        // Try YAML first, then JSON
        let yaml_path = dir.join("bonsai-extension.yaml");
        let json_path = dir.join("extension.json");
        let alt_yaml = dir.join(".bonsai").join("extension.yaml");

        if yaml_path.exists() {
            let s = std::fs::read_to_string(&yaml_path)
                .map_err(|e| ExtensionError::Io(e.to_string()))?;
            serde_yaml::from_str(&s)
                .map_err(|e| ExtensionError::InvalidManifest(e.to_string()))
        } else if alt_yaml.exists() {
            let s = std::fs::read_to_string(&alt_yaml)
                .map_err(|e| ExtensionError::Io(e.to_string()))?;
            serde_yaml::from_str(&s)
                .map_err(|e| ExtensionError::InvalidManifest(e.to_string()))
        } else if json_path.exists() {
            let s = std::fs::read_to_string(&json_path)
                .map_err(|e| ExtensionError::Io(e.to_string()))?;
            serde_json::from_str(&s)
                .map_err(|e| ExtensionError::InvalidManifest(e.to_string()))
        } else {
            Err(ExtensionError::InvalidManifest(
                "no bonsai-extension.yaml or extension.json found in repo root".into(),
            ))
        }
    }

    async fn copy_extension_files(
        &self,
        src: &Path,
        dst: &Path,
    ) -> Result<(), ExtensionError> {
        for entry in walkdir::WalkDir::new(src)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let rel = entry
                .path()
                .strip_prefix(src)
                .unwrap_or(entry.path());
            let dest = dst.join(rel);

            if entry.file_type().is_dir() {
                tokio::fs::create_dir_all(&dest)
                    .await
                    .map_err(|e| ExtensionError::Io(e.to_string()))?;
            } else {
                tokio::fs::copy(entry.path(), &dest)
                    .await
                    .map_err(|e| ExtensionError::Io(e.to_string()))?;
            }
        }
        Ok(())
    }
}
