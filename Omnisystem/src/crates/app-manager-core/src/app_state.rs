use crate::{AppId, Version};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppState {
    Discovered,
    Downloading,
    Downloaded,
    Verifying,
    Verified,
    Installing,
    Installed,
    Loading,
    Loaded,
    Running,
    Stopped,
    Unloading,
    Uninstalling,
    Uninstalled,
    Failed,
    Corrupted,
}

impl AppState {
    pub fn is_stable(&self) -> bool {
        matches!(
            self,
            AppState::Installed | AppState::Loaded | AppState::Running | AppState::Stopped
        )
    }

    pub fn is_transitioning(&self) -> bool {
        matches!(
            self,
            AppState::Downloading
                | AppState::Verifying
                | AppState::Installing
                | AppState::Loading
                | AppState::Unloading
                | AppState::Uninstalling
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(self, AppState::Failed | AppState::Corrupted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub app_id: AppId,
    pub version: Version,
    pub state: AppState,

    pub installation_path: PathBuf,
    pub config_path: PathBuf,
    pub data_path: PathBuf,

    pub dependencies: Vec<(AppId, Version)>,
    pub dependents: Vec<AppId>,

    pub installed_at: DateTime<Utc>,
    pub last_started: Option<DateTime<Utc>>,
    pub last_stopped: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,

    pub auto_start: bool,
    pub auto_update: bool,

    pub size_bytes: u64,
    pub last_error: Option<String>,
}

impl InstalledApp {
    pub fn new(app_id: AppId, version: Version, installation_path: PathBuf) -> Self {
        let now = Utc::now();
        InstalledApp {
            app_id,
            version,
            state: AppState::Installed,
            installation_path,
            config_path: PathBuf::new(),
            data_path: PathBuf::new(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            installed_at: now,
            last_started: None,
            last_stopped: None,
            updated_at: now,
            auto_start: false,
            auto_update: false,
            size_bytes: 0,
            last_error: None,
        }
    }

    pub fn set_state(&mut self, state: AppState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    pub fn mark_started(&mut self) {
        self.state = AppState::Running;
        self.last_started = Some(Utc::now());
    }

    pub fn mark_stopped(&mut self) {
        self.state = AppState::Stopped;
        self.last_stopped = Some(Utc::now());
    }

    pub fn mark_error(&mut self, error: String) {
        self.state = AppState::Failed;
        self.last_error = Some(error);
    }

    pub fn add_dependency(&mut self, app_id: AppId, version: Version) {
        self.dependencies.push((app_id, version));
    }

    pub fn add_dependent(&mut self, app_id: AppId) {
        if !self.dependents.contains(&app_id) {
            self.dependents.push(app_id);
        }
    }

    pub fn remove_dependent(&mut self, app_id: &AppId) {
        self.dependents.retain(|d| d != app_id);
    }

    pub fn is_running(&self) -> bool {
        self.state == AppState::Running
    }

    pub fn is_installed(&self) -> bool {
        self.state == AppState::Installed || self.state == AppState::Loaded || self.state == AppState::Running
    }

    pub fn uptime(&self) -> Option<chrono::Duration> {
        self.last_started.map(|started| Utc::now().signed_duration_since(started))
    }

    pub fn age(&self) -> chrono::Duration {
        Utc::now().signed_duration_since(self.installed_at)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub app_id: AppId,
    pub version: Version,
    pub timestamp: DateTime<Utc>,
    pub app_data: Vec<u8>,
    pub config_data: Vec<u8>,
}

impl AppSnapshot {
    pub fn new(app_id: AppId, version: Version) -> Self {
        AppSnapshot {
            app_id,
            version,
            timestamp: Utc::now(),
            app_data: Vec::new(),
            config_data: Vec::new(),
        }
    }

    pub fn save_app_data(&mut self, data: Vec<u8>) {
        self.app_data = data;
    }

    pub fn save_config_data(&mut self, data: Vec<u8>) {
        self.config_data = data;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_installed_app_creation() {
        let app = InstalledApp::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        assert_eq!(app.version, Version::new(1, 0, 0));
        assert_eq!(app.state, AppState::Installed);
        assert!(app.is_installed());
    }

    #[test]
    fn test_app_state_transitions() {
        let mut app = InstalledApp::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        assert!(!app.is_running());

        app.set_state(AppState::Loaded);
        assert_eq!(app.state, AppState::Loaded);

        app.mark_started();
        assert!(app.is_running());

        app.mark_stopped();
        assert!(!app.is_running());
    }

    #[test]
    fn test_dependency_management() {
        let mut app = InstalledApp::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        app.add_dependency(AppId::new("dep1").unwrap(), Version::new(1, 0, 0));
        app.add_dependency(AppId::new("dep2").unwrap(), Version::new(2, 0, 0));

        assert_eq!(app.dependencies.len(), 2);
    }

    #[test]
    fn test_dependent_management() {
        let mut app = InstalledApp::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        let dependent = AppId::new("dependent").unwrap();
        app.add_dependent(dependent.clone());
        assert_eq!(app.dependents.len(), 1);

        app.remove_dependent(&dependent);
        assert_eq!(app.dependents.len(), 0);
    }

    #[test]
    fn test_error_marking() {
        let mut app = InstalledApp::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
            PathBuf::from("/apps/test-app"),
        );

        app.mark_error("Something went wrong".to_string());
        assert_eq!(app.state, AppState::Failed);
        assert!(app.last_error.is_some());
    }

    #[test]
    fn test_app_snapshot() {
        let mut snapshot = AppSnapshot::new(
            AppId::new("test-app").unwrap(),
            Version::new(1, 0, 0),
        );

        snapshot.save_app_data(vec![1, 2, 3, 4, 5]);
        snapshot.save_config_data(vec![10, 20, 30]);

        assert_eq!(snapshot.app_data.len(), 5);
        assert_eq!(snapshot.config_data.len(), 3);
    }
}
