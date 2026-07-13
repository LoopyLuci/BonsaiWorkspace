use crate::{AppId, Version, Result, AppManagerError};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Alpha,
    Development,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: Version,
    pub channel: ReleaseChannel,
    pub released_at: DateTime<Utc>,
    pub changelog: String,
    pub breaking_changes: bool,
    pub deprecated_features: Vec<String>,
}

impl VersionInfo {
    pub fn new(version: Version, channel: ReleaseChannel) -> Self {
        VersionInfo {
            version,
            channel,
            released_at: Utc::now(),
            changelog: String::new(),
            breaking_changes: false,
            deprecated_features: Vec::new(),
        }
    }

    pub fn set_changelog(&mut self, changelog: String) -> &mut Self {
        self.changelog = changelog;
        self
    }

    pub fn mark_breaking(&mut self) -> &mut Self {
        self.breaking_changes = true;
        self
    }

    pub fn add_deprecated_feature(&mut self, feature: String) -> &mut Self {
        self.deprecated_features.push(feature);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    pub app_id: AppId,
    pub versions: Vec<VersionInfo>,
    pub current_version: Version,
    pub previous_version: Option<Version>,
}

impl VersionHistory {
    pub fn new(app_id: AppId, current_version: Version) -> Self {
        VersionHistory {
            app_id,
            versions: vec![VersionInfo::new(current_version.clone(), ReleaseChannel::Stable)],
            current_version,
            previous_version: None,
        }
    }

    pub fn add_version(&mut self, version_info: VersionInfo) {
        self.versions.push(version_info);
        self.versions.sort_by(|a, b| b.version.cmp(&a.version));
    }

    pub fn update_version(&mut self, version: Version) -> Result<()> {
        if self.current_version == version {
            return Err(AppManagerError::Internal("Already at this version".to_string()));
        }

        self.previous_version = Some(self.current_version.clone());
        self.current_version = version;
        Ok(())
    }

    pub fn get_version(&self, version: &Version) -> Option<VersionInfo> {
        self.versions.iter().find(|v| v.version == *version).cloned()
    }

    pub fn rollback_to(&mut self, version: Version) -> Result<()> {
        if !self.versions.iter().any(|v| v.version == version) {
            return Err(AppManagerError::AppNotFound(format!(
                "Version {} not found",
                version
            )));
        }

        self.previous_version = Some(self.current_version.clone());
        self.current_version = version;
        Ok(())
    }

    pub fn latest_stable(&self) -> Option<VersionInfo> {
        self.versions
            .iter()
            .find(|v| v.channel == ReleaseChannel::Stable)
            .cloned()
    }

    pub fn latest_beta(&self) -> Option<VersionInfo> {
        self.versions
            .iter()
            .find(|v| v.channel == ReleaseChannel::Beta)
            .cloned()
    }

    pub fn all_versions(&self) -> Vec<VersionInfo> {
        self.versions.clone()
    }
}

pub struct VersionManager {
    histories: HashMap<AppId, VersionHistory>,
}

impl VersionManager {
    pub fn new() -> Self {
        VersionManager {
            histories: HashMap::new(),
        }
    }

    pub fn create_history(&mut self, app_id: AppId, version: Version) {
        self.histories
            .insert(app_id.clone(), VersionHistory::new(app_id, version));
    }

    pub fn add_version(&mut self, app_id: &AppId, version_info: VersionInfo) -> Result<()> {
        match self.histories.get_mut(app_id) {
            Some(history) => {
                history.add_version(version_info);
                Ok(())
            }
            None => Err(AppManagerError::AppNotFound(app_id.to_string())),
        }
    }

    pub fn update_to_version(&mut self, app_id: &AppId, version: Version) -> Result<()> {
        match self.histories.get_mut(app_id) {
            Some(history) => history.update_version(version),
            None => Err(AppManagerError::AppNotFound(app_id.to_string())),
        }
    }

    pub fn get_history(&self, app_id: &AppId) -> Result<VersionHistory> {
        self.histories
            .get(app_id)
            .cloned()
            .ok_or_else(|| AppManagerError::AppNotFound(app_id.to_string()))
    }

    pub fn rollback(&mut self, app_id: &AppId, version: Version) -> Result<()> {
        match self.histories.get_mut(app_id) {
            Some(history) => history.rollback_to(version),
            None => Err(AppManagerError::AppNotFound(app_id.to_string())),
        }
    }

    pub fn get_latest_stable(&self, app_id: &AppId) -> Result<VersionInfo> {
        let history = self.get_history(app_id)?;
        history
            .latest_stable()
            .ok_or_else(|| AppManagerError::AppNotFound("No stable version".to_string()))
    }

    pub fn is_update_available(&self, app_id: &AppId) -> Result<bool> {
        let history = self.get_history(app_id)?;
        if let Some(latest) = history.latest_stable() {
            Ok(latest.version > history.current_version)
        } else {
            Ok(false)
        }
    }

    pub fn get_current_version(&self, app_id: &AppId) -> Result<Version> {
        let history = self.get_history(app_id)?;
        Ok(history.current_version)
    }

    pub fn check_breaking_changes(&self, app_id: &AppId, from_version: &Version, to_version: &Version) -> Result<bool> {
        let history = self.get_history(app_id)?;
        let versions_between = history
            .versions
            .iter()
            .filter(|v| v.version > *from_version && v.version <= *to_version)
            .any(|v| v.breaking_changes);

        Ok(versions_between)
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info_creation() {
        let info = VersionInfo::new(Version::new(1, 0, 0), ReleaseChannel::Stable);

        assert_eq!(info.version, Version::new(1, 0, 0));
        assert_eq!(info.channel, ReleaseChannel::Stable);
        assert_eq!(info.changelog, "");
    }

    #[test]
    fn test_version_history() {
        let mut history = VersionHistory::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
        );

        history.add_version(VersionInfo::new(Version::new(1, 1, 0), ReleaseChannel::Stable));
        history.add_version(VersionInfo::new(Version::new(1, 2, 0), ReleaseChannel::Beta));

        assert_eq!(history.all_versions().len(), 3);
    }

    #[test]
    fn test_version_update() {
        let mut history = VersionHistory::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
        );

        history.update_version(Version::new(1, 1, 0)).unwrap();
        assert_eq!(history.current_version, Version::new(1, 1, 0));
        assert_eq!(history.previous_version, Some(Version::new(1, 0, 0)));
    }

    #[test]
    fn test_rollback() {
        let mut history = VersionHistory::new(
            AppId::new("test-app").unwrap(),
            Version::new(2, 0, 0),
        );

        history.add_version(VersionInfo::new(Version::new(1, 0, 0), ReleaseChannel::Stable));
        history.rollback_to(Version::new(1, 0, 0)).unwrap();

        assert_eq!(history.current_version, Version::new(1, 0, 0));
    }

    #[test]
    fn test_version_manager() {
        let mut manager = VersionManager::new();
        let app_id = AppId::new("test-app").unwrap();

        manager.create_history(app_id.clone(), Version::new(1, 0, 0));

        let info = VersionInfo::new(Version::new(1, 1, 0), ReleaseChannel::Stable);
        manager.add_version(&app_id, info).unwrap();

        let history = manager.get_history(&app_id).unwrap();
        assert_eq!(history.all_versions().len(), 2);
    }

    #[test]
    fn test_is_update_available() {
        let mut manager = VersionManager::new();
        let app_id = AppId::new("test-app").unwrap();

        manager.create_history(app_id.clone(), Version::new(1, 0, 0));
        manager.add_version(&app_id, VersionInfo::new(Version::new(1, 1, 0), ReleaseChannel::Stable))
            .unwrap();

        assert!(manager.is_update_available(&app_id).unwrap());
    }
}
