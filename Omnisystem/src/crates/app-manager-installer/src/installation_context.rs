use app_manager_core::{AppId, Version, Manifest};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationContext {
    pub app_id: AppId,
    pub version: Version,
    pub installation_path: PathBuf,
    pub config_path: PathBuf,
    pub data_path: PathBuf,

    pub manifest: Option<Manifest>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub phases: Vec<String>,

    pub installed: bool,
    pub progress_percent: u32,
}

impl InstallationContext {
    pub fn new(app_id: AppId, version: Version, installation_path: PathBuf) -> Self {
        let config_path = installation_path.join("config");
        let data_path = installation_path.join("data");

        InstallationContext {
            app_id,
            version,
            installation_path,
            config_path,
            data_path,
            manifest: None,
            start_time: Utc::now(),
            end_time: None,
            phases: Vec::new(),
            installed: false,
            progress_percent: 0,
        }
    }

    pub fn set_manifest(&mut self, manifest: Manifest) {
        self.manifest = Some(manifest);
    }

    pub fn mark_phase(&mut self, phase: &str) {
        self.phases.push(phase.to_string());
        self.progress_percent = ((self.phases.len() as u32) * 25).min(100);
    }

    pub fn mark_installed(&mut self) {
        self.installed = true;
        self.end_time = Some(Utc::now());
        self.progress_percent = 100;
    }

    pub fn duration(&self) -> chrono::Duration {
        let end = self.end_time.unwrap_or_else(Utc::now);
        end.signed_duration_since(self.start_time)
    }

    pub fn is_complete(&self) -> bool {
        self.installed
    }

    pub fn get_elapsed_seconds(&self) -> i64 {
        self.duration().num_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_installation_context_creation() {
        let context = InstallationContext::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        assert!(!context.installed);
        assert_eq!(context.progress_percent, 0);
    }

    #[test]
    fn test_mark_phase() {
        let mut context = InstallationContext::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        context.mark_phase("phase1");
        assert_eq!(context.phases.len(), 1);
        assert!(context.progress_percent > 0);
    }

    #[test]
    fn test_mark_installed() {
        let mut context = InstallationContext::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        context.mark_installed();
        assert!(context.installed);
        assert_eq!(context.progress_percent, 100);
        assert!(context.end_time.is_some());
    }

    #[test]
    fn test_duration() {
        let mut context = InstallationContext::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        context.mark_installed();
        let duration = context.duration();
        assert!(duration.num_seconds() >= 0);
    }
}
