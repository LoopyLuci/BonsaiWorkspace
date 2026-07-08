use universe::{EventCategory, EventSource, UniverseEvent};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequestEvent {
    pub request_id: Uuid,
    pub method: String,
    pub path: String,
    pub client_ip: String,
    pub peer_id: Option<String>,
    pub status_code: u16,
    pub duration_ms: u64,
    pub capability_used: String,
}

#[derive(Clone)]
pub struct TelemetryBus {
    tx: broadcast::Sender<String>,
}

impl TelemetryBus {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(2048);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    pub fn emit_api_event(&self, event: ApiRequestEvent) {
        let universe_event = UniverseEvent::new(
            EventSource::System {
                component: "bonsai-api-bridge".to_string(),
            },
            EventCategory::ComputeEvent,
            format!("API {} {} -> {}", event.method, event.path, event.status_code),
            event.path.clone(),
            "api-bridge-device",
        )
        .with_metadata(serde_json::to_value(&event).unwrap_or(serde_json::Value::Null));

        let line = serde_json::json!({
            "api": event,
            "universe": universe_event,
        })
        .to_string();

        let _ = self.tx.send(line);
    }
}
