use crate::system_event_bus::{SharedEventBus, SystemEvent};
use std::collections::HashMap;
use std::sync::Arc;
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
}

impl UpgradeDispatcher {
    pub fn new(event_bus: SharedEventBus) -> Self {
        Self {
            event_bus,
            version_ledger: Arc::new(RwLock::new(HashMap::new())),
            health_check_url: "http://127.0.0.1:11420/health".to_string(),
            rollback_grace_secs: 60,
        }
    }

    pub fn start(self: Arc<Self>) {
        let dispatcher = self.clone();
        tokio::spawn(async move {
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

    async fn perform_upgrade(&self, component: &str, _version: &str, _cas_hash: &str) -> bool {
        // Model reloads delegate to hot_reload.rs (already implemented).
        // WASM tool swaps, binary blue-green, and UI panel reloads are wired here
        // as each mechanism is built (P0-A-1 through P0-A-3).
        match component {
            "model" | "adapter" => {
                info!("Upgrade: model/adapter swap delegated to hot_reload");
                true
            }
            _ => {
                info!("Upgrade: no-op handler for component '{}'", component);
                true
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
