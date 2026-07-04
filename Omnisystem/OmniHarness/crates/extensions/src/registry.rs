//! Extension Registry — tracks installed extensions, their config, and user allowlists.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    manifest::{ExtensionCategory, ExtensionManifest, SecurityVerdict},
    scanner::SecurityReport,
};

/// The status of an installed extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionStatus {
    /// Active and contributing to the ecosystem.
    Enabled,
    /// Installed but not active.
    Disabled,
    /// Security review pending.
    PendingReview,
    /// Failed to load.
    Error,
}

/// An installed extension entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub id: Uuid,
    pub manifest: ExtensionManifest,
    pub status: ExtensionStatus,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Path where the extension files live: `~/.bonsai/extensions/{extension_id}/`.
    pub install_path: PathBuf,
    /// Path to the cloned source (for fork/edit).
    pub source_path: Option<PathBuf>,
    /// User-overridden configuration values.
    pub config: HashMap<String, serde_json::Value>,
    /// Pattern findings the user has explicitly allowed.
    pub user_allowlist: Vec<String>,
    /// Latest security report (kept for the UI).
    pub last_security_report: Option<SecurityReport>,
    /// Whether updates have been checked.
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub install_count: u64,
    pub user_rating: Option<u8>,
}

impl InstalledExtension {
    pub fn new(manifest: ExtensionManifest, install_path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            manifest,
            status: ExtensionStatus::PendingReview,
            installed_at: now,
            updated_at: now,
            install_path,
            source_path: None,
            config: HashMap::new(),
            user_allowlist: Vec::new(),
            last_security_report: None,
            update_available: false,
            latest_version: None,
            install_count: 0,
            user_rating: None,
        }
    }

    /// Merge a config value. Returns error if the key is not in the schema.
    pub fn set_config(&mut self, key: &str, value: serde_json::Value) -> Result<(), String> {
        if !self.manifest.config_schema.contains_key(key) {
            return Err(format!("unknown config key: {key}"));
        }
        self.config.insert(key.to_string(), value);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Reset all config to defaults from the schema.
    pub fn reset_config(&mut self) {
        self.config.clear();
        self.updated_at = Utc::now();
    }

    pub fn effective_verdict(&self) -> SecurityVerdict {
        self.last_security_report
            .as_ref()
            .map(|r| r.verdict)
            .unwrap_or(SecurityVerdict::Unreviewed)
    }
}

/// Browseable extension card — a lightweight summary for the Browse/Discover tab.
/// Populated from remote gossip or the community index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCard {
    pub extension_id: String,
    pub name: String,
    pub description: String,
    pub author_name: String,
    pub repository: String,
    pub category: ExtensionCategory,
    pub tags: Vec<String>,
    pub version: String,
    pub verdict: SecurityVerdict,
    pub risk_score: u8,
    pub install_count: u64,
    pub rating: f32,
    pub icon: Option<String>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// Thread-safe in-memory extension registry.
///
/// In production, this persists to SQLite under `~/.bonsai/extensions/registry.db`.
/// For now it uses an in-memory DashMap.
#[derive(Clone)]
pub struct ExtensionRegistry {
    /// extension_id -> InstalledExtension
    installed: Arc<DashMap<String, InstalledExtension>>,
    /// extension_id -> ExtensionCard (discovered but not installed)
    discovered: Arc<DashMap<String, ExtensionCard>>,
    /// Base directory for all extension installations.
    pub base_dir: PathBuf,
}

impl ExtensionRegistry {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            installed: Arc::new(DashMap::new()),
            discovered: Arc::new(DashMap::new()),
            base_dir: base_dir.into(),
        }
    }

    /// Install or update an extension.
    pub fn register(&self, ext: InstalledExtension) {
        self.installed.insert(ext.manifest.extension_id.clone(), ext);
    }

    /// Uninstall an extension.
    pub fn uninstall(&self, extension_id: &str) {
        self.installed.remove(extension_id);
    }

    /// Enable/disable a named extension.
    pub fn set_status(&self, extension_id: &str, status: ExtensionStatus) -> bool {
        if let Some(mut e) = self.installed.get_mut(extension_id) {
            e.status = status;
            e.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Update config for an extension.
    pub fn set_config(
        &self,
        extension_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        self.installed
            .get_mut(extension_id)
            .ok_or_else(|| format!("extension {extension_id} not found"))?
            .set_config(key, value)
    }

    /// Attach the security report to an installed extension.
    pub fn attach_report(&self, extension_id: &str, report: SecurityReport) {
        if let Some(mut e) = self.installed.get_mut(extension_id) {
            e.status = match report.verdict {
                SecurityVerdict::Blocked => ExtensionStatus::Disabled,
                _ => ExtensionStatus::Enabled,
            };
            e.last_security_report = Some(report);
            e.updated_at = Utc::now();
        }
    }

    /// Mark a finding as user-allowed.
    pub fn allow_finding(&self, extension_id: &str, finding_technical: &str) {
        if let Some(mut e) = self.installed.get_mut(extension_id) {
            if !e.user_allowlist.contains(&finding_technical.to_string()) {
                e.user_allowlist.push(finding_technical.to_string());
            }
        }
    }

    /// Get a snapshot of all installed extensions.
    pub fn list_installed(&self) -> Vec<InstalledExtension> {
        self.installed.iter().map(|e| e.value().clone()).collect()
    }

    /// Get a single installed extension.
    pub fn get_installed(&self, extension_id: &str) -> Option<InstalledExtension> {
        self.installed.get(extension_id).map(|e| e.clone())
    }

    /// Add a discovered (not yet installed) extension card.
    pub fn add_discovered(&self, card: ExtensionCard) {
        self.discovered.insert(card.extension_id.clone(), card);
    }

    /// All discovered extension cards (Browse tab).
    pub fn list_discovered(&self) -> Vec<ExtensionCard> {
        self.discovered.iter().map(|e| e.value().clone()).collect()
    }

    /// All extensions — installed overrides discovered for the same id.
    pub fn list_all_cards(&self) -> Vec<ExtensionCard> {
        let mut cards: Vec<ExtensionCard> = self
            .discovered
            .iter()
            .map(|e| e.value().clone())
            .collect();

        // Overlay installed extensions (they may have updated verdicts)
        for ext in self.list_installed() {
            let card = ExtensionCard {
                extension_id: ext.manifest.extension_id.clone(),
                name: ext.manifest.name.clone(),
                description: ext.manifest.description.clone(),
                author_name: ext.manifest.author.name.clone(),
                repository: ext.manifest.repository.clone(),
                category: ext.manifest.category,
                tags: ext.manifest.tags.clone(),
                version: ext.manifest.version.clone(),
                verdict: ext.effective_verdict(),
                risk_score: ext
                    .last_security_report
                    .as_ref()
                    .map(|r| r.risk_score)
                    .unwrap_or(0),
                install_count: ext.install_count,
                rating: ext.user_rating.map(|r| r as f32).unwrap_or(0.0),
                icon: ext.manifest.icon.clone(),
                last_updated: Some(ext.updated_at),
            };
            if let Some(pos) = cards.iter().position(|c| c.extension_id == card.extension_id) {
                cards[pos] = card;
            } else {
                cards.push(card);
            }
        }
        cards
    }

    pub fn installed_count(&self) -> usize {
        self.installed.len()
    }
}
