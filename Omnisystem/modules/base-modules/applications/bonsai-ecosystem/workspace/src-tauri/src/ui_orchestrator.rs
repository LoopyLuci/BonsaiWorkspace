use cas::{CasKey, CasStore};
use crate::system_event_bus::{SharedEventBus, SystemEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPanel {
    pub id: String,
    pub description: String,
    pub cas_key: String,
    pub created_at_ms: u64,
}

pub struct UiOrchestrator {
    event_bus: SharedEventBus,
    cas: Arc<CasStore>,
    panels: RwLock<HashMap<String, UiPanel>>,
}

impl UiOrchestrator {
    pub fn new(event_bus: SharedEventBus, cas: Arc<CasStore>) -> Self {
        Self {
            event_bus,
            cas,
            panels: RwLock::new(HashMap::new()),
        }
    }

    pub async fn generate_panel(&self, description: String) -> Result<UiPanel, String> {
        let now = std::time::SystemTime::now();
        let created_at_ms = now
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64;

        let panel_id = uuid::Uuid::new_v4().to_string();
        let manifest = json!({
            "id": panel_id,
            "description": description,
            "created_at_ms": created_at_ms,
            "schema_version": 1,
            "components": [],
        });
        let payload = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
        let key = self
            .cas
            .put(&payload, "application/vnd.bonsai.ui-panel+json")
            .await
            .map_err(|e| e.to_string())?;
        let cas_hash = key.hex();
        self.cas.pin(&key).await.map_err(|e| e.to_string())?;

        let panel = UiPanel {
            id: panel_id.clone(),
            description: description.clone(),
            cas_key: cas_hash.clone(),
            created_at_ms,
        };

        self.panels.write().await.insert(panel_id.clone(), panel.clone());
        self.event_bus.publish(SystemEvent::UiPanelGenerated {
            panel_id,
            description,
            cas_hash,
        });
        Ok(panel)
    }

    pub async fn list_panels(&self) -> Vec<UiPanel> {
        self.panels
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    pub async fn reload_panel(&self, panel_id: &str) -> Result<(), String> {
        let panels = self.panels.read().await;
        let panel = panels
            .get(panel_id)
            .ok_or_else(|| format!("UI panel not found: {}", panel_id))?
            .clone();

        self.event_bus.publish(SystemEvent::UiPanelReloadRequested {
            panel_id: panel.id,
            cas_hash: panel.cas_key,
        });
        Ok(())
    }
}
