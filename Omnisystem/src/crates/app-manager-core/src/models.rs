//! Shared data models

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::app::AppId;

/// Installation record tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationRecord {
    pub id: String,
    pub app_id: AppId,
    pub installed_at: DateTime<Utc>,
    pub version: semver::Version,
    pub location: std::path::PathBuf,
    pub status: InstallationStatus,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum InstallationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Uninstalled,
}

impl InstallationRecord {
    pub fn new(app_id: AppId, version: semver::Version, location: std::path::PathBuf) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            app_id,
            installed_at: Utc::now(),
            version,
            location,
            status: InstallationStatus::Pending,
        }
    }

    pub fn mark_in_progress(&mut self) {
        self.status = InstallationStatus::InProgress;
    }

    pub fn mark_completed(&mut self) {
        self.status = InstallationStatus::Completed;
    }

    pub fn mark_failed(&mut self) {
        self.status = InstallationStatus::Failed;
    }
}

/// Marketplace listing for an app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceListing {
    pub app_id: AppId,
    pub title: String,
    pub description: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub rating: f32,
    pub review_count: u32,
    pub download_count: u32,
    pub free: bool,
    pub price_cents: Option<u32>,
    pub banner_url: String,
    pub created_at: DateTime<Utc>,
}

impl MarketplaceListing {
    pub fn new(app_id: AppId, title: String, category: String, banner_url: String) -> Self {
        Self {
            app_id,
            title,
            description: String::new(),
            category,
            subcategory: None,
            rating: 0.0,
            review_count: 0,
            download_count: 0,
            free: true,
            price_cents: None,
            banner_url,
            created_at: Utc::now(),
        }
    }
}

/// User review for an app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserReview {
    pub id: String,
    pub app_id: AppId,
    pub user_id: String,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub helpful_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserReview {
    pub fn new(app_id: AppId, user_id: String, rating: u8, title: String, content: String) -> Option<Self> {
        if !(1..=5).contains(&rating) {
            return None;
        }

        Some(Self {
            id: Uuid::new_v4().to_string(),
            app_id,
            user_id,
            rating,
            title,
            content,
            helpful_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

/// Version information for tracking releases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: semver::Version,
    pub released_at: DateTime<Utc>,
    pub changelog: String,
    pub prerelease: bool,
    pub deprecated: bool,
}

impl VersionInfo {
    pub fn new(version: semver::Version, changelog: String) -> Self {
        let prerelease = !version.pre.is_empty();
        Self {
            version,
            released_at: Utc::now(),
            changelog,
            prerelease,
            deprecated: false,
        }
    }

    pub fn mark_deprecated(&mut self) {
        self.deprecated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installation_record_creation() {
        let app_id = AppId::new();
        let record = InstallationRecord::new(
            app_id.clone(),
            semver::Version::new(1, 0, 0),
            std::path::PathBuf::from("/apps/test"),
        );

        assert_eq!(record.app_id, app_id);
        assert_eq!(record.status, InstallationStatus::Pending);
    }

    #[test]
    fn test_installation_record_lifecycle() {
        let app_id = AppId::new();
        let mut record = InstallationRecord::new(
            app_id,
            semver::Version::new(1, 0, 0),
            std::path::PathBuf::from("/apps/test"),
        );

        record.mark_in_progress();
        assert_eq!(record.status, InstallationStatus::InProgress);

        record.mark_completed();
        assert_eq!(record.status, InstallationStatus::Completed);
    }

    #[test]
    fn test_user_review_rating_validation() {
        let app_id = AppId::new();

        assert!(UserReview::new(
            app_id.clone(),
            "user1".to_string(),
            5,
            "Great!".to_string(),
            "Really good".to_string()
        ).is_some());

        assert!(UserReview::new(
            app_id.clone(),
            "user1".to_string(),
            0,
            "Bad".to_string(),
            "Not valid".to_string()
        ).is_none());

        assert!(UserReview::new(
            app_id,
            "user1".to_string(),
            6,
            "Too high".to_string(),
            "Not valid".to_string()
        ).is_none());
    }

    #[test]
    fn test_version_info_prerelease_detection() {
        let pre_version = semver::Version::parse("1.0.0-alpha.1").unwrap();
        let stable_version = semver::Version::parse("1.0.0").unwrap();

        let pre_info = VersionInfo::new(pre_version, "Alpha release".to_string());
        assert!(pre_info.prerelease);

        let stable_info = VersionInfo::new(stable_version, "Stable release".to_string());
        assert!(!stable_info.prerelease);
    }
}
