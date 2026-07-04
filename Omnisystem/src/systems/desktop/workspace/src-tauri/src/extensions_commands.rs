//! Tauri command layer for the Bonsai Extensions System.
//!
//! Exposes install, scan, configure, uninstall, and browse operations
//! to the frontend as typed IPC commands.

use extensions::{
    default_registry,
    installer::{InstallResult, Installer},
    manifest::{ExtensionCategory, ExtensionManifest, SecurityVerdict},
    registry::{ExtensionCard, ExtensionRegistry, ExtensionStatus, InstalledExtension},
    scanner::{Finding, SecurityReport, Severity},
    ExtensionError,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

// ── App-managed state ──────────────────────────────────────────────────────────

pub struct ExtensionsState {
    pub registry: ExtensionRegistry,
}

impl ExtensionsState {
    pub fn new() -> Self {
        Self { registry: default_registry() }
    }
}

// ── DTOs ───────────────────────────────────────────────────────────────────────

/// Slim card for the Browse grid.
#[derive(Serialize)]
pub struct ExtensionCardDto {
    pub extension_id: String,
    pub name: String,
    pub description: String,
    pub author_name: String,
    pub repository: String,
    pub category: String,
    pub tags: Vec<String>,
    pub version: String,
    pub verdict: String,
    pub risk_score: u8,
    pub install_count: u64,
    pub rating: f32,
    pub icon: Option<String>,
    pub is_installed: bool,
}

impl From<ExtensionCard> for ExtensionCardDto {
    fn from(c: ExtensionCard) -> Self {
        Self {
            extension_id: c.extension_id,
            name: c.name,
            description: c.description,
            author_name: c.author_name,
            repository: c.repository,
            category: c.category.to_string(),
            tags: c.tags,
            version: c.version,
            verdict: c.verdict.to_string(),
            risk_score: c.risk_score,
            install_count: c.install_count,
            rating: c.rating,
            icon: c.icon,
            is_installed: false, // resolved at query time
        }
    }
}

/// Full installed extension detail for the Installed tab.
#[derive(Serialize)]
pub struct InstalledExtensionDto {
    pub extension_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author_name: String,
    pub repository: String,
    pub category: String,
    pub status: String,
    pub verdict: String,
    pub risk_score: u8,
    pub installed_at: String,
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub config: serde_json::Value,
    pub config_schema: serde_json::Value,
    pub install_path: String,
    pub has_source: bool,
    pub finding_summary: FindingSummaryDto,
}

#[derive(Serialize, Default)]
pub struct FindingSummaryDto {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl From<InstalledExtension> for InstalledExtensionDto {
    fn from(e: InstalledExtension) -> Self {
        let verdict = e.effective_verdict().to_string();
        let risk_score = e.last_security_report.as_ref().map(|r| r.risk_score).unwrap_or(0);
        let finding_summary = e.last_security_report.as_ref().map(|r| FindingSummaryDto {
            critical: r.critical_count(),
            high: r.high_count(),
            medium: r.medium_count(),
            low: r.low_count(),
        }).unwrap_or_default();

        InstalledExtensionDto {
            extension_id: e.manifest.extension_id.clone(),
            name: e.manifest.name,
            version: e.manifest.version,
            description: e.manifest.description,
            author_name: e.manifest.author.name,
            repository: e.manifest.repository,
            category: e.manifest.category.to_string(),
            status: format!("{:?}", e.status),
            verdict,
            risk_score,
            installed_at: e.installed_at.to_rfc3339(),
            update_available: e.update_available,
            latest_version: e.latest_version,
            config: serde_json::to_value(&e.config).unwrap_or(serde_json::Value::Null),
            config_schema: serde_json::to_value(&e.manifest.config_schema)
                .unwrap_or(serde_json::Value::Null),
            install_path: e.install_path.to_string_lossy().to_string(),
            has_source: e.source_path.is_some(),
            finding_summary,
        }
    }
}

// ── Commands ───────────────────────────────────────────────────────────────────

/// Install an extension from a GitHub repository URL.
/// Returns the install result and triggers a security scan.
#[tauri::command]
pub async fn ext_install_from_github(
    state: State<'_, ExtensionsState>,
    github_url: String,
) -> Result<InstallResult, String> {
    let installer = Installer::new(state.registry.clone());
    installer
        .install_from_github(&github_url)
        .await
        .map_err(|e| e.to_string())
}

/// Install an extension from a local directory path.
#[tauri::command]
pub async fn ext_install_from_path(
    state: State<'_, ExtensionsState>,
    dir_path: String,
) -> Result<InstallResult, String> {
    let installer = Installer::new(state.registry.clone());
    installer
        .install_from_dir(&PathBuf::from(&dir_path), Some(&dir_path))
        .await
        .map_err(|e| e.to_string())
}

/// Uninstall an extension by id.
#[tauri::command]
pub async fn ext_uninstall(
    state: State<'_, ExtensionsState>,
    extension_id: String,
) -> Result<(), String> {
    let ext = state
        .registry
        .get_installed(&extension_id)
        .ok_or_else(|| format!("extension {extension_id} not found"))?;

    // Remove files
    let _ = tokio::fs::remove_dir_all(&ext.install_path).await;
    state.registry.uninstall(&extension_id);
    Ok(())
}

/// Enable or disable an installed extension.
#[tauri::command]
pub async fn ext_set_enabled(
    state: State<'_, ExtensionsState>,
    extension_id: String,
    enabled: bool,
) -> Result<(), String> {
    let status = if enabled { ExtensionStatus::Enabled } else { ExtensionStatus::Disabled };
    if !state.registry.set_status(&extension_id, status) {
        return Err(format!("extension {extension_id} not found"));
    }
    Ok(())
}

/// Update a single config value for an extension.
#[tauri::command]
pub async fn ext_set_config(
    state: State<'_, ExtensionsState>,
    extension_id: String,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    state.registry.set_config(&extension_id, &key, value)
}

/// Reset all config to defaults for an extension.
#[tauri::command]
pub async fn ext_reset_config(
    state: State<'_, ExtensionsState>,
    extension_id: String,
) -> Result<(), String> {
    state
        .registry
        .get_installed(&extension_id)
        .ok_or_else(|| format!("extension {extension_id} not found"))?;
    // Re-register with reset config
    if let Some(mut ext) = state.registry.get_installed(&extension_id) {
        ext.reset_config();
        state.registry.register(ext);
    }
    Ok(())
}

/// Allow a specific security finding for an extension (user override).
#[tauri::command]
pub async fn ext_allow_finding(
    state: State<'_, ExtensionsState>,
    extension_id: String,
    finding_technical: String,
) -> Result<(), String> {
    state.registry.allow_finding(&extension_id, &finding_technical);
    Ok(())
}

/// Re-scan an already-installed extension for security.
#[tauri::command]
pub async fn ext_rescan(
    state: State<'_, ExtensionsState>,
    extension_id: String,
) -> Result<SecurityReport, String> {
    let ext = state
        .registry
        .get_installed(&extension_id)
        .ok_or_else(|| format!("extension {extension_id} not found"))?;

    let scanner = extensions::SecurityScanner::new();
    let report = scanner
        .scan(
            &ext.install_path,
            &ext.manifest.extension_id,
            &ext.manifest.version,
        )
        .await;

    state.registry.attach_report(&extension_id, report.clone());
    Ok(report)
}

/// Get the full security report for an installed extension.
#[tauri::command]
pub async fn ext_get_security_report(
    state: State<'_, ExtensionsState>,
    extension_id: String,
) -> Result<Option<SecurityReport>, String> {
    let ext = state
        .registry
        .get_installed(&extension_id)
        .ok_or_else(|| format!("extension {extension_id} not found"))?;
    Ok(ext.last_security_report)
}

/// List all installed extensions.
#[tauri::command]
pub async fn ext_list_installed(
    state: State<'_, ExtensionsState>,
) -> Result<Vec<InstalledExtensionDto>, String> {
    Ok(state
        .registry
        .list_installed()
        .into_iter()
        .map(InstalledExtensionDto::from)
        .collect())
}

/// List all extension cards (installed + discovered).
#[tauri::command]
pub async fn ext_list_all(
    state: State<'_, ExtensionsState>,
) -> Result<Vec<ExtensionCardDto>, String> {
    let installed_ids: std::collections::HashSet<String> = state
        .registry
        .list_installed()
        .iter()
        .map(|e| e.manifest.extension_id.clone())
        .collect();

    let cards = state
        .registry
        .list_all_cards()
        .into_iter()
        .map(|c| {
            let is_installed = installed_ids.contains(&c.extension_id);
            let mut dto = ExtensionCardDto::from(c);
            dto.is_installed = is_installed;
            dto
        })
        .collect();

    Ok(cards)
}

/// Get full detail for one installed extension.
#[tauri::command]
pub async fn ext_get_detail(
    state: State<'_, ExtensionsState>,
    extension_id: String,
) -> Result<InstalledExtensionDto, String> {
    state
        .registry
        .get_installed(&extension_id)
        .map(InstalledExtensionDto::from)
        .ok_or_else(|| format!("extension {extension_id} not installed"))
}

/// Scan a remote GitHub URL without installing (for the preview/detail page).
#[tauri::command]
pub async fn ext_preview_scan(
    state: State<'_, ExtensionsState>,
    github_url: String,
) -> Result<SecurityReport, String> {
    // Lightweight: only download and scan, don't install
    let url_hash = {
        let mut h = blake3::Hasher::new();
        h.update(github_url.as_bytes());
        hex::encode(&h.finalize().as_bytes()[..8])
    };
    let tmp_dir = state
        .registry
        .base_dir
        .join("_scan_tmp")
        .join(&url_hash);

    let archive_url = if github_url.ends_with(".git") {
        format!("{}/archive/refs/heads/main.zip", github_url.trim_end_matches(".git"))
    } else {
        format!("{}/archive/refs/heads/main.zip", github_url.trim_end_matches('/'))
    };

    let resp = reqwest::get(&archive_url)
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} fetching repo", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;

    tokio::fs::create_dir_all(&tmp_dir).await.map_err(|e| e.to_string())?;
    let archive_path = tmp_dir.join("archive.zip");
    tokio::fs::write(&archive_path, &bytes).await.map_err(|e| e.to_string())?;

    let extract_dir = tmp_dir.join("extracted");
    tokio::fs::create_dir_all(&extract_dir).await.map_err(|e| e.to_string())?;
    let ap = archive_path.clone();
    let ed = extract_dir.clone();
    tokio::task::spawn_blocking(move || {
        let f = std::fs::File::open(&ap)?;
        let mut a = zip::ZipArchive::new(f)?;
        a.extract(&ed)?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: Box<dyn std::error::Error + Send + Sync>| e.to_string())?;

    let mut repo_dir = extract_dir.clone();
    let mut rd = tokio::fs::read_dir(&extract_dir).await.map_err(|e| e.to_string())?;
    if let Ok(Some(entry)) = rd.next_entry().await {
        repo_dir = entry.path();
    }

    let scanner = extensions::SecurityScanner::new();
    let report = scanner.scan(&repo_dir, "preview", "0.0.0").await;

    let _ = tokio::fs::remove_dir_all(&tmp_dir).await;
    Ok(report)
}

/// User rates an extension 1–5 stars.
#[tauri::command]
pub async fn ext_rate(
    state: State<'_, ExtensionsState>,
    extension_id: String,
    rating: u8,
) -> Result<(), String> {
    if rating == 0 || rating > 5 {
        return Err("rating must be 1–5".into());
    }
    if let Some(mut ext) = state.registry.get_installed(&extension_id) {
        ext.user_rating = Some(rating);
        state.registry.register(ext);
    }
    Ok(())
}
