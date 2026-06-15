//! Application State Management
//!
//! Manages services list, environments tree, user session, and cache state.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use omni_bot_core::{ServiceInfo, ServiceState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Environment snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub size_mb: u64,
    pub hash: String,
}

/// Environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub is_running: bool,
    pub snapshots: Vec<EnvironmentSnapshot>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl EnvironmentInfo {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            parent_id: None,
            created_at: now,
            modified_at: now,
            is_running: false,
            snapshots: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent_id: String) -> Self {
        self.parent_id = Some(parent_id);
        self
    }
}

/// Current session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub token: Option<String>,
    pub is_authenticated: bool,
}

impl SessionInfo {
    pub fn new(user_id: String) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4().to_string(),
            user_id,
            created_at: now,
            last_activity: now,
            token: None,
            is_authenticated: false,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token.clone());
        self.is_authenticated = true;
        self
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// Module information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub installed_at: DateTime<Utc>,
    pub is_enabled: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ModuleInfo {
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            installed_at: Utc::now(),
            is_enabled: true,
            metadata: HashMap::new(),
        }
    }
}

/// Application state container
pub struct AppState {
    services: DashMap<String, ServiceInfo>,
    environments: DashMap<String, EnvironmentInfo>,
    modules: DashMap<String, ModuleInfo>,
    session: parking_lot::Mutex<SessionInfo>,
    online: parking_lot::Mutex<bool>,
    sync_version: parking_lot::Mutex<u64>,
}

impl AppState {
    /// Create a new application state
    pub fn new(user_id: String) -> Self {
        Self {
            services: DashMap::new(),
            environments: DashMap::new(),
            modules: DashMap::new(),
            session: parking_lot::Mutex::new(SessionInfo::new(user_id)),
            online: parking_lot::Mutex::new(true),
            sync_version: parking_lot::Mutex::new(0),
        }
    }

    // ========== Service Management ==========

    /// Add or update a service
    pub fn upsert_service(&self, service: ServiceInfo) -> Result<()> {
        self.services.insert(service.name.clone(), service);
        self.increment_sync_version();
        Ok(())
    }

    /// Get service by name
    pub fn get_service(&self, name: &str) -> Result<Option<ServiceInfo>> {
        Ok(self.services.get(name).map(|entry| entry.clone()))
    }

    /// List all services
    pub fn list_services(&self) -> Result<Vec<ServiceInfo>> {
        Ok(self
            .services
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    /// Remove a service
    pub fn remove_service(&self, name: &str) -> Result<()> {
        self.services.remove(name);
        self.increment_sync_version();
        Ok(())
    }

    /// Get services by state
    pub fn services_by_state(&self, state: ServiceState) -> Result<Vec<ServiceInfo>> {
        Ok(self
            .services
            .iter()
            .filter(|entry| entry.value().state == state)
            .map(|entry| entry.value().clone())
            .collect())
    }

    // ========== Environment Management ==========

    /// Add or update an environment
    pub fn upsert_environment(&self, env: EnvironmentInfo) -> Result<()> {
        self.environments.insert(env.id.clone(), env);
        self.increment_sync_version();
        Ok(())
    }

    /// Get environment by ID
    pub fn get_environment(&self, id: &str) -> Result<Option<EnvironmentInfo>> {
        Ok(self.environments.get(id).map(|entry| entry.clone()))
    }

    /// List all environments
    pub fn list_environments(&self) -> Result<Vec<EnvironmentInfo>> {
        Ok(self
            .environments
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    /// List environments by parent (tree hierarchy)
    pub fn list_child_environments(&self, parent_id: &str) -> Result<Vec<EnvironmentInfo>> {
        Ok(self
            .environments
            .iter()
            .filter(|entry| {
                entry
                    .value()
                    .parent_id
                    .as_ref()
                    .map(|p| p == parent_id)
                    .unwrap_or(false)
            })
            .map(|entry| entry.value().clone())
            .collect())
    }

    /// Remove an environment
    pub fn remove_environment(&self, id: &str) -> Result<()> {
        self.environments.remove(id);
        self.increment_sync_version();
        Ok(())
    }

    /// Add snapshot to environment
    pub fn add_snapshot(&self, env_id: &str, snapshot: EnvironmentSnapshot) -> Result<()> {
        if let Some(mut env) = self.environments.get_mut(env_id) {
            env.snapshots.push(snapshot);
            env.modified_at = Utc::now();
            self.increment_sync_version();
            Ok(())
        } else {
            Err(Error::NotFound(format!("Environment {} not found", env_id)))
        }
    }

    // ========== Module Management ==========

    /// Install a module
    pub fn install_module(&self, module: ModuleInfo) -> Result<()> {
        self.modules.insert(module.name.clone(), module);
        self.increment_sync_version();
        Ok(())
    }

    /// Get module by name
    pub fn get_module(&self, name: &str) -> Result<Option<ModuleInfo>> {
        Ok(self.modules.get(name).map(|entry| entry.clone()))
    }

    /// List all installed modules
    pub fn list_modules(&self) -> Result<Vec<ModuleInfo>> {
        Ok(self
            .modules
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    /// Remove a module
    pub fn remove_module(&self, name: &str) -> Result<()> {
        self.modules.remove(name);
        self.increment_sync_version();
        Ok(())
    }

    // ========== Session Management ==========

    /// Get current session
    pub fn get_session(&self) -> Result<SessionInfo> {
        Ok(self.session.lock().clone())
    }

    /// Update session
    pub fn update_session<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut SessionInfo),
    {
        let mut session = self.session.lock();
        f(&mut session);
        self.increment_sync_version();
        Ok(())
    }

    /// Authenticate session
    pub fn authenticate(&self, token: String) -> Result<()> {
        let mut session = self.session.lock();
        session.token = Some(token);
        session.is_authenticated = true;
        session.update_activity();
        self.increment_sync_version();
        Ok(())
    }

    // ========== Connectivity ==========

    /// Set online status
    pub fn set_online(&self, online: bool) {
        *self.online.lock() = online;
        self.increment_sync_version();
    }

    /// Get online status
    pub fn is_online(&self) -> bool {
        *self.online.lock()
    }

    // ========== Sync Version ==========

    /// Increment sync version for change tracking
    pub fn increment_sync_version(&self) {
        let mut version = self.sync_version.lock();
        *version = version.saturating_add(1);
    }

    /// Get current sync version
    pub fn get_sync_version(&self) -> u64 {
        *self.sync_version.lock()
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "services_count": self.services.len(),
            "environments_count": self.environments.len(),
            "modules_count": self.modules.len(),
            "online": self.is_online(),
            "sync_version": self.get_sync_version(),
            "session": self.get_session()?
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_creation() {
        let state = AppState::new("test_user".to_string());
        let session = state.get_session().unwrap();
        assert_eq!(session.user_id, "test_user");
        assert!(!session.is_authenticated);
    }

    #[test]
    fn test_environment_creation() {
        let state = AppState::new("test_user".to_string());
        let env = EnvironmentInfo::new("dev_env".to_string());
        state.upsert_environment(env.clone()).unwrap();

        let retrieved = state.get_environment(&env.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "dev_env");
    }

    #[test]
    fn test_environment_hierarchy() {
        let state = AppState::new("test_user".to_string());
        let parent = EnvironmentInfo::new("parent".to_string());
        let parent_id = parent.id.clone();
        state.upsert_environment(parent).unwrap();

        let child = EnvironmentInfo::new("child".to_string()).with_parent(parent_id.clone());
        state.upsert_environment(child).unwrap();

        let children = state.list_child_environments(&parent_id).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "child");
    }

    #[test]
    fn test_sync_version() {
        let state = AppState::new("test_user".to_string());
        let v1 = state.get_sync_version();

        state.upsert_environment(EnvironmentInfo::new("env1".to_string())).unwrap();
        let v2 = state.get_sync_version();

        assert!(v2 > v1);
    }
}
