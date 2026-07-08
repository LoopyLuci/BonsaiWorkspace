//! Tauri Command Handlers
//!
//! IPC bridge for Tauri frontend with offline-first fallbacks.
//! Implements service management, environment operations, and module installation.

use crate::api_client::{ApiClient, ApiClientConfig};
use crate::cache::CacheManager;
use crate::error::Result;
use crate::offline_queue::OfflineQueue;
use crate::state::{AppState, EnvironmentInfo, ModuleInfo};
use crate::sync_engine::SyncEngine;
use omni_bot_core::{Action, ServiceInfo};
use serde_json::{json, Value};
use std::sync::Arc;

/// Command handlers with full system integration
pub struct CommandHandlers {
    state: Arc<AppState>,
    api_client: Arc<ApiClient>,
    cache: Arc<CacheManager>,
    queue: Arc<OfflineQueue>,
    sync_engine: Arc<SyncEngine>,
}

impl CommandHandlers {
    /// Initialize handlers with all subsystems
    pub fn new(user_id: String) -> Result<Self> {
        let state = Arc::new(AppState::new(user_id));
        let api_client = Arc::new(ApiClient::new(ApiClientConfig::default()));
        let cache = Arc::new(CacheManager::new(1000, 3600));
        let queue = Arc::new(OfflineQueue::new());
        let sync_engine = Arc::new(SyncEngine::new("buddy-app".to_string()));

        Ok(Self {
            state,
            api_client,
            cache,
            queue,
            sync_engine,
        })
    }

    /// Initialize with custom components
    pub fn new_with_config(
        state: Arc<AppState>,
        api_client: Arc<ApiClient>,
        cache: Arc<CacheManager>,
        queue: Arc<OfflineQueue>,
        sync_engine: Arc<SyncEngine>,
    ) -> Self {
        Self {
            state,
            api_client,
            cache,
            queue,
            sync_engine,
        }
    }

    // ========== Service Management ==========

    /// Start a service
    pub async fn start_service(
        &self,
        name: String,
        config: Option<Value>,
    ) -> Result<ServiceInfo> {
        if !self.state.is_online() {
            // Offline mode: queue the action
            let mut metadata = std::collections::HashMap::new();
            if let Some(cfg) = config {
                metadata.insert("config".to_string(), cfg);
            }
            let action = Action::StartService {
                name: name.clone(),
                config: if metadata.is_empty() { None } else { Some(metadata) },
            };
            self.queue.enqueue(action)?;
            return Err(crate::error::Error::Offline(
                "Queued action for syncing when online".to_string(),
            ));
        }

        match self.api_client.start_service(&name, config).await {
            Ok(service) => {
                self.state.upsert_service(service.clone())?;
                self.cache.put(
                    format!("service:{}", name),
                    serde_json::to_value(&service)?,
                )?;
                Ok(service)
            }
            Err(e) => {
                // Fallback to cache if available
                if let Ok(Some(cached)) = self.cache.get(&format!("service:{}", name)) {
                    let service: ServiceInfo = serde_json::from_value(cached)?;
                    Ok(service)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Stop a service
    pub async fn stop_service(&self, name: String, force: bool) -> Result<ServiceInfo> {
        if !self.state.is_online() {
            let action = Action::StopService {
                name: name.clone(),
                force,
            };
            self.queue.enqueue(action)?;
            return Err(crate::error::Error::Offline(
                "Queued action for syncing when online".to_string(),
            ));
        }

        match self.api_client.stop_service(&name, force).await {
            Ok(service) => {
                self.state.upsert_service(service.clone())?;
                self.cache.invalidate(&format!("service:{}", name))?;
                Ok(service)
            }
            Err(e) => {
                if let Ok(Some(cached)) = self.cache.get(&format!("service:{}", name)) {
                    let service: ServiceInfo = serde_json::from_value(cached)?;
                    Ok(service)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Get service status
    pub async fn get_service_status(&self, name: String) -> Result<ServiceInfo> {
        // Try cache first
        let cache_key = format!("service:{}", name);
        if let Ok(Some(cached)) = self.cache.get(&cache_key) {
            if let Ok(service) = serde_json::from_value::<ServiceInfo>(cached) {
                return Ok(service);
            }
        }

        // Try API
        if self.state.is_online() {
            match self.api_client.get_service(&name).await {
                Ok(service) => {
                    self.state.upsert_service(service.clone())?;
                    self.cache.put(cache_key, serde_json::to_value(&service)?)?;
                    return Ok(service);
                }
                Err(e) => {
                    log::warn!("API call failed: {}, checking local state", e);
                }
            }
        }

        // Fall back to local state
        if let Ok(Some(service)) = self.state.get_service(&name) {
            Ok(service)
        } else {
            Err(crate::error::Error::NotFound(format!(
                "Service {} not found",
                name
            )))
        }
    }

    /// List all services
    pub async fn list_services(&self) -> Result<Vec<ServiceInfo>> {
        // Try cache first
        if let Ok(Some(cached)) = self.cache.get("services:list") {
            if let Ok(services) = serde_json::from_value::<Vec<ServiceInfo>>(cached) {
                return Ok(services);
            }
        }

        // Try API
        if self.state.is_online() {
            match self.api_client.list_services().await {
                Ok(services) => {
                    for service in &services {
                        self.state.upsert_service(service.clone())?;
                    }
                    self.cache.put("services:list".to_string(), serde_json::to_value(&services)?)?;
                    return Ok(services);
                }
                Err(e) => {
                    log::warn!("API call failed: {}, using local state", e);
                }
            }
        }

        // Fall back to local state
        self.state.list_services()
    }

    // ========== Environment Management ==========

    /// Create a new environment
    pub async fn create_environment(&self, name: String) -> Result<EnvironmentInfo> {
        let env = EnvironmentInfo::new(name);
        self.state.upsert_environment(env.clone())?;

        if self.state.is_online() {
            let mut spec = std::collections::HashMap::new();
            spec.insert(
                "id".to_string(),
                serde_json::Value::String(env.id.clone()),
            );
            spec.insert(
                "created_at".to_string(),
                serde_json::Value::String(env.created_at.to_rfc3339()),
            );
            let action = Action::CreateEnvironment {
                name: env.name.clone(),
                spec,
            };
            let _ = self.api_client.execute_action(action).await;
        }

        Ok(env)
    }

    /// List all environments
    pub async fn list_environments(&self) -> Result<Vec<EnvironmentInfo>> {
        self.state.list_environments()
    }

    /// Get environment by ID
    pub async fn get_environment(&self, id: String) -> Result<EnvironmentInfo> {
        self.state
            .get_environment(&id)?
            .ok_or_else(|| crate::error::Error::NotFound(format!("Environment {} not found", id)))
    }

    /// Delete environment
    pub async fn delete_environment(&self, id: String, _force: bool) -> Result<()> {
        self.state.remove_environment(&id)?;

        if self.state.is_online() {
            let action = Action::DeleteEnvironment { id: id.clone() };
            let _ = self.api_client.execute_action(action).await;
        }

        Ok(())
    }

    /// Create environment snapshot
    pub async fn snapshot_environment(&self, id: String, name: String) -> Result<String> {
        use blake3::Hasher;

        // Generate hash
        let mut hasher = Hasher::new();
        hasher.update(id.as_bytes());
        hasher.update(chrono::Utc::now().to_rfc3339().as_bytes());
        let hash = hasher.finalize().to_hex().to_string();

        let snapshot_name = name.clone();
        let snapshot = crate::state::EnvironmentSnapshot {
            id: hash.clone(),
            name: snapshot_name,
            created_at: chrono::Utc::now(),
            size_mb: 0,
            hash: hash.clone(),
        };

        self.state.add_snapshot(&id, snapshot)?;

        if self.state.is_online() {
            let action = Action::SnapshotEnvironment {
                id: id.clone(),
                name,
            };
            let _ = self.api_client.execute_action(action).await;
        }

        Ok(hash)
    }

    // ========== Module Management ==========

    /// Install a module
    pub async fn install_module(&self, name: String, version: String) -> Result<ModuleInfo> {
        if !self.state.is_online() {
            let action = Action::InstallModule {
                name: name.clone(),
                version: version.clone(),
            };
            self.queue.enqueue(action)?;
            return Err(crate::error::Error::Offline(
                "Module installation queued for online sync".to_string(),
            ));
        }

        let action = Action::InstallModule {
            name: name.clone(),
            version: version.clone(),
        };

        let _ = self.api_client.execute_action(action).await?;

        let module = ModuleInfo::new(name, version);
        self.state.install_module(module.clone())?;
        Ok(module)
    }

    /// List installed modules
    pub async fn list_modules(&self) -> Result<Vec<ModuleInfo>> {
        self.state.list_modules()
    }

    /// Remove a module
    pub async fn remove_module(&self, name: String) -> Result<()> {
        if self.state.is_online() {
            let action = Action::RemoveModule { name: name.clone() };
            let _ = self.api_client.execute_action(action).await;
        }

        self.state.remove_module(&name)
    }

    /// Search for modules
    pub async fn search_modules(&self, query: String) -> Result<Vec<Value>> {
        if !self.state.is_online() {
            return Err(crate::error::Error::Offline(
                "Module search requires online connection".to_string(),
            ));
        }

        match self.api_client.search_modules(&query).await {
            Ok(results) => {
                self.cache.put(
                    format!("search:modules:{}", query),
                    serde_json::to_value(&results)?,
                )?;
                Ok(results)
            }
            Err(e) => {
                if let Ok(Some(cached)) = self.cache.get(&format!("search:modules:{}", query)) {
                    serde_json::from_value::<Vec<Value>>(cached).map_err(|_| e)
                } else {
                    Err(e)
                }
            }
        }
    }

    // ========== System Operations ==========

    /// Get application summary
    pub async fn get_summary(&self) -> Result<Value> {
        self.state.get_summary()
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<Value> {
        let stats = self.cache.stats();
        Ok(json!({
            "hits": stats.hits,
            "misses": stats.misses,
            "hit_rate": stats.hit_rate(),
            "size": stats.size,
            "evictions": stats.evictions,
        }))
    }

    /// Get queue statistics
    pub async fn get_queue_stats(&self) -> Result<Value> {
        Ok(self.queue.stats())
    }

    /// Get sync engine statistics
    pub async fn get_sync_stats(&self) -> Result<Value> {
        Ok(self.sync_engine.stats())
    }

    /// Set online status
    pub async fn set_online(&self, online: bool) -> Result<()> {
        self.state.set_online(online);

        if online {
            // Process queued actions on reconnection
            self.process_offline_queue().await?;
        }

        Ok(())
    }

    /// Process offline queue on reconnection
    async fn process_offline_queue(&self) -> Result<()> {
        while let Ok(Some(action)) = self.queue.dequeue() {
            match self.api_client.execute_action(action.action.clone()).await {
                Ok(_) => {
                    self.queue.mark_synced(&action.id, true, None)?;
                }
                Err(e) => {
                    log::warn!("Failed to sync action {}: {}", action.id, e);
                    if action.can_retry() {
                        // Re-queue for retry
                        let mut retry_action = action.clone();
                        retry_action.increment_retry();
                        // In real impl, would re-add to queue
                    } else {
                        self.queue.mark_synced(&action.id, false, Some(e.to_string()))?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Clear offline queue
    pub async fn clear_queue(&self) -> Result<()> {
        self.queue.clear()
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<()> {
        self.cache.clear()
    }

    /// Cleanup expired cache entries
    pub async fn cleanup_cache(&self) -> Result<usize> {
        self.cache.cleanup_expired()
    }

    /// Get full state snapshot for debugging
    pub async fn get_debug_snapshot(&self) -> Result<Value> {
        Ok(json!({
            "summary": self.state.get_summary()?,
            "cache_stats": self.get_cache_stats().await?,
            "queue_stats": self.get_queue_stats().await?,
            "sync_stats": self.get_sync_stats().await?,
            "online": self.state.is_online(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handlers_creation() {
        let result = CommandHandlers::new("test_user".to_string());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_environment() {
        let handlers = CommandHandlers::new("test_user".to_string()).unwrap();
        let env = handlers.create_environment("dev_env".to_string()).await.unwrap();

        assert_eq!(env.name, "dev_env");
        assert!(!env.is_running);
    }

    #[tokio::test]
    async fn test_list_environments() {
        let handlers = CommandHandlers::new("test_user".to_string()).unwrap();
        handlers.create_environment("env1".to_string()).await.unwrap();
        handlers.create_environment("env2".to_string()).await.unwrap();

        let envs = handlers.list_environments().await.unwrap();
        assert_eq!(envs.len(), 2);
    }

    #[tokio::test]
    async fn test_install_module() {
        let handlers = CommandHandlers::new("test_user".to_string()).unwrap();
        handlers.state.set_online(false); // Offline mode

        let result = handlers
            .install_module("test_module".to_string(), "1.0.0".to_string())
            .await;

        // Should fail in offline mode but queue the action
        assert!(result.is_err());
        assert!(!handlers.queue.is_empty());
    }

    #[tokio::test]
    async fn test_snapshot_environment() {
        let handlers = CommandHandlers::new("test_user".to_string()).unwrap();
        let env = handlers.create_environment("snap_env".to_string()).await.unwrap();

        let hash = handlers
            .snapshot_environment(env.id.clone(), "snap1".to_string())
            .await
            .unwrap();

        assert!(!hash.is_empty());

        let updated_env = handlers.get_environment(env.id).await.unwrap();
        assert_eq!(updated_env.snapshots.len(), 1);
    }

    #[tokio::test]
    async fn test_get_summary() {
        let handlers = CommandHandlers::new("test_user".to_string()).unwrap();
        let summary = handlers.get_summary().await.unwrap();

        assert!(summary.get("services_count").is_some());
        assert!(summary.get("online").is_some());
    }
}
