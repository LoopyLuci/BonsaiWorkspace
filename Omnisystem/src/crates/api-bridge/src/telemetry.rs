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

impl Default for TelemetryBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> ApiRequestEvent {
        ApiRequestEvent {
            request_id: Uuid::new_v4(),
            method: "GET".to_string(),
            path: "/api/v1/inference".to_string(),
            client_ip: "127.0.0.1".to_string(),
            peer_id: None,
            status_code: 200,
            duration_ms: 42,
            capability_used: "ApiCap:inference".to_string(),
        }
    }

    #[tokio::test]
    async fn test_subscribers_receive_emitted_events() {
        let bus = TelemetryBus::new();
        let mut rx = bus.subscribe();

        bus.emit_api_event(sample_event());

        let line = rx.recv().await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed["api"]["status_code"], 200);
        assert_eq!(parsed["api"]["method"], "GET");
        // The universe event summary should mention the path and status.
        assert!(parsed["universe"]["summary"]
            .as_str()
            .unwrap()
            .contains("/api/v1/inference"));
    }

    #[tokio::test]
    async fn test_emit_without_subscribers_does_not_panic() {
        let bus = TelemetryBus::new();
        bus.emit_api_event(sample_event()); // no subscribers; send() returns Err, ignored
    }
}
