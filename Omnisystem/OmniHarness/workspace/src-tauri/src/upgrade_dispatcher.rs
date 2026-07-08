use crate::system_event_bus::{SharedEventBus, SystemEvent};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentVersion {
    pub component: String,
    pub version: String,
    pub cas_hash: String,
    pub deployed_at: std::time::SystemTime,
    pub health_check_passed: bool,
    pub previous_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpgradePolicy {
    Manual,
    AutoOnGreenCi,
    Canary { hours: u32 },
}

pub struct UpgradeDispatcher {
    event_bus: SharedEventBus,
    version_ledger: Arc<RwLock<HashMap<String, Vec<ComponentVersion>>>>,
    health_check_url: String,
    rollback_grace_secs: u64,
    /// Needed to actually materialize a CAS blob and perform a model/adapter
    /// hot-swap in `perform_upgrade` — without these, "upgrade" for a model
    /// component was previously just a log line that unconditionally returned
    /// `true` with no real effect (see `perform_upgrade` history).
    app_handle: AppHandle,
    orchestrator: Arc<crate::model_orchestrator::ModelOrchestrator>,
    cas_store: Arc<cas::CasStore>,
}

impl UpgradeDispatcher {
    pub fn new(
        event_bus: SharedEventBus,
        app_handle: AppHandle,
        orchestrator: Arc<crate::model_orchestrator::ModelOrchestrator>,
        cas_store: Arc<cas::CasStore>,
    ) -> Self {
        Self {
            event_bus,
            version_ledger: Arc::new(RwLock::new(HashMap::new())),
            health_check_url: format!("http://127.0.0.1:{}/health", crate::config::BUDDY_API_PORT),
            rollback_grace_secs: 60,
            app_handle,
            orchestrator,
            cas_store,
        }
    }

    pub fn start(self: Arc<Self>) {
        let dispatcher = self.clone();
        // `start()` is called synchronously from Tauri's non-async `.setup()`
        // closure (see `lib.rs`) — raw `tokio::spawn` panics there with "no
        // reactor running" since there's no ambient Tokio runtime at that
        // call site; `tauri::async_runtime::spawn` enters Tauri's runtime
        // instead. Same class of bug fixed elsewhere in this file's siblings.
        tauri::async_runtime::spawn(async move {
            let mut rx = dispatcher.event_bus.subscribe();
            while let Ok(event) = rx.recv().await {
                if let SystemEvent::UpgradeReady { component, version, cas_hash, source: _ } = event {
                    let d = dispatcher.clone();
                    tokio::spawn(async move {
                        d.handle_upgrade_ready(component, version, cas_hash).await;
                    });
                }
            }
        });
    }

    async fn handle_upgrade_ready(&self, component: String, version: String, cas_hash: String) {
        info!("Upgrade ready: {} v{}", component, version);
        self.event_bus.publish(SystemEvent::UpgradeDeploying {
            component: component.clone(),
            version: version.clone(),
        });

        let start = std::time::Instant::now();
        let success = self.perform_upgrade(&component, &version, &cas_hash).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        if success && self.health_check(self.rollback_grace_secs).await {
            let mut ledger = self.version_ledger.write().await;
            let history = ledger.entry(component.clone()).or_default();
            let previous = history.last().map(|v| v.version.clone());
            history.push(ComponentVersion {
                component: component.clone(),
                version: version.clone(),
                cas_hash,
                deployed_at: std::time::SystemTime::now(),
                health_check_passed: true,
                previous_version: previous,
            });
            self.event_bus.publish(SystemEvent::UpgradeDeployed { component, version, duration_ms });
        } else {
            let previous_version = {
                let ledger = self.version_ledger.read().await;
                ledger.get(&component)
                    .and_then(|h| h.last())
                    .map(|v| v.version.clone())
                    .unwrap_or_else(|| "unknown".into())
            };
            warn!("Upgrade failed health check, rolling back {}", component);
            self.event_bus.publish(SystemEvent::UpgradeRolledBack {
                component,
                reason: "Health check failed within grace period".into(),
                previous_version,
            });
        }
    }

    async fn perform_upgrade(&self, component: &str, version: &str, cas_hash: &str) -> bool {
        // WASM tool swaps, binary blue-green, and UI panel reloads are wired here
        // as each mechanism is built (P0-A-1 through P0-A-3).
        match component {
            "model" | "adapter" => self.perform_model_upgrade(component, version, cas_hash).await,
            _ => {
                info!("Upgrade: no-op handler for component '{}'", component);
                true
            }
        }
    }

    /// Materialize the CAS blob for a model/adapter upgrade to disk, then run
    /// it through the same zero-downtime swap `hot_reload`'s file-watcher uses.
    async fn perform_model_upgrade(&self, component: &str, version: &str, cas_hash: &str) -> bool {
        let key = match cas::CasKey::from_hex(cas_hash) {
            Ok(k) => k,
            Err(e) => {
                warn!("Upgrade: invalid cas_hash for {component} v{version}: {e}");
                return false;
            }
        };
        let bytes = match self.cas_store.get(&key).await {
            Ok(Some(b)) => b,
            Ok(None) => {
                warn!("Upgrade: cas_hash {cas_hash} not found in CAS store for {component}");
                return false;
            }
            Err(e) => {
                warn!("Upgrade: CAS read failed for {component} v{version}: {e}");
                return false;
            }
        };

        let models_dir = self
            .app_handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("models");
        if let Err(e) = std::fs::create_dir_all(&models_dir) {
            warn!("Upgrade: could not create models dir: {e}");
            return false;
        }
        let model_id = format!("{component}-{version}");
        let staged_path = models_dir.join(format!("{model_id}.gguf"));
        if let Err(e) = std::fs::write(&staged_path, &bytes) {
            warn!("Upgrade: could not write staged model file: {e}");
            return false;
        }

        let previous_model_id = {
            let ledger = self.version_ledger.read().await;
            ledger
                .get(component)
                .and_then(|h| h.last())
                .map(|v| format!("{component}-{}", v.version))
        };

        match crate::hot_reload::reload_model(
            &self.app_handle,
            &self.orchestrator,
            model_id,
            &staged_path,
            previous_model_id,
        )
        .await
        {
            Ok(()) => true,
            Err(e) => {
                warn!("Upgrade: model swap failed for {component} v{version}: {e}");
                false
            }
        }
    }

    async fn health_check(&self, grace_secs: u64) -> bool {
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(grace_secs);
        loop {
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            if let Ok(resp) = reqwest::get(&self.health_check_url).await {
                if resp.status().is_success() {
                    return true;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    pub async fn get_version_history(&self, component: &str) -> Vec<ComponentVersion> {
        let ledger = self.version_ledger.read().await;
        ledger.get(component).cloned().unwrap_or_default()
    }

    pub async fn rollback(&self, component: &str) {
        let ledger = self.version_ledger.read().await;
        if let Some(history) = ledger.get(component) {
            let len = history.len();
            if len >= 2 {
                let prev = &history[len - 2];
                info!("Rolling back {} to v{}", component, prev.version);
                self.event_bus.publish(SystemEvent::UpgradeReady {
                    component: component.to_string(),
                    version: prev.version.clone(),
                    cas_hash: prev.cas_hash.clone(),
                    source: "manual-rollback".into(),
                });
            } else {
                warn!("No previous version found for {}", component);
            }
        }
    }
}
