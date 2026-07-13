use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Telemetry event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryEventType {
    /// Device discovery
    DeviceDiscovered,
    /// Connection established
    Connected,
    /// Connection lost
    Disconnected,
    /// Authentication success
    AuthSuccess,
    /// Authentication failure
    AuthFailure,
    /// Screen frame captured
    FrameCaptured,
    /// Input event injected
    InputInjected,
    /// File synced
    FileSynced,
    /// Capability granted
    CapabilityGranted,
    /// Capability revoked
    CapabilityRevoked,
    /// Error occurred
    Error,
    /// Performance metric
    Metric,
}

/// Telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: TelemetryEventType,
    /// Device ID (if applicable)
    pub device_id: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event data (as JSON)
    pub data: serde_json::Value,
    /// User/agent identifier
    pub agent_id: Option<String>,
    /// Severity level
    pub severity: String,
}

impl TelemetryEvent {
    /// Create new telemetry event
    pub fn new(
        event_type: TelemetryEventType,
        device_id: Option<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            device_id,
            timestamp: chrono::Utc::now(),
            data,
            agent_id: None,
            severity: "info".to_string(),
        }
    }

    /// Set severity
    pub fn with_severity(mut self, severity: &str) -> Self {
        self.severity = severity.to_string();
        self
    }

    /// Set agent ID
    pub fn with_agent(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
}

/// Telemetry collector
pub struct TelemetryCollector {
    /// Event buffer
    events: Arc<parking_lot::RwLock<Vec<TelemetryEvent>>>,
    /// Event sender
    event_tx: tokio::sync::mpsc::UnboundedSender<TelemetryEvent>,
    /// Max buffer size
    max_buffer_size: usize,
}

impl TelemetryCollector {
    /// Create new telemetry collector
    pub fn new(
        event_tx: tokio::sync::mpsc::UnboundedSender<TelemetryEvent>,
        max_buffer_size: usize,
    ) -> Self {
        Self {
            events: Arc::new(parking_lot::RwLock::new(Vec::new())),
            event_tx,
            max_buffer_size,
        }
    }

    /// Record telemetry event
    pub fn record(&self, event: TelemetryEvent) {
        // Send to external collector (W&B, etc.)
        let _ = self.event_tx.send(event.clone());

        // Buffer locally
        let mut events = self.events.write();
        events.push(event);

        // Trim old events if buffer gets too large
        if events.len() > self.max_buffer_size {
            let drain_count = events.len() - self.max_buffer_size;
            events.drain(0..drain_count);
        }
    }

    /// Get recent events
    pub fn get_recent_events(&self, count: usize) -> Vec<TelemetryEvent> {
        let events = self.events.read();
        let start = if events.len() > count {
            events.len() - count
        } else {
            0
        };
        events[start..].to_vec()
    }

    /// Get events by device
    pub fn get_events_by_device(&self, device_id: &str, count: usize) -> Vec<TelemetryEvent> {
        let events = self.events.read();
        events
            .iter()
            .filter(|e| e.device_id.as_deref() == Some(device_id))
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Clear buffer
    pub fn clear(&self) {
        self.events.write().clear();
    }

    /// Get statistics
    pub fn get_stats(&self) -> TelemetryStats {
        let events = self.events.read();
        let mut stats = TelemetryStats::default();

        for event in events.iter() {
            match event.event_type {
                TelemetryEventType::DeviceDiscovered => stats.devices_discovered += 1,
                TelemetryEventType::Connected => stats.connections_established += 1,
                TelemetryEventType::Disconnected => stats.disconnections += 1,
                TelemetryEventType::AuthSuccess => stats.auth_success += 1,
                TelemetryEventType::AuthFailure => stats.auth_failures += 1,
                TelemetryEventType::FrameCaptured => stats.frames_captured += 1,
                TelemetryEventType::InputInjected => stats.inputs_injected += 1,
                TelemetryEventType::FileSynced => stats.files_synced += 1,
                TelemetryEventType::Error => stats.errors += 1,
                _ => {}
            }
        }

        stats
    }
}

/// Telemetry statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryStats {
    pub devices_discovered: u64,
    pub connections_established: u64,
    pub disconnections: u64,
    pub auth_success: u64,
    pub auth_failures: u64,
    pub frames_captured: u64,
    pub inputs_injected: u64,
    pub files_synced: u64,
    pub errors: u64,
}

/// Universe bridge for integration with Bonsai Universe
pub struct UniverseBridge {
    /// Universe event emitter
    universe_tx: tokio::sync::mpsc::UnboundedSender<UniverseEvent>,
}

/// Universe event for time-travel debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseEvent {
    /// Event hash
    pub hash: String,
    /// Event type
    pub event_type: String,
    /// Event data
    pub data: serde_json::Value,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Parent hash (for causality)
    pub parent_hash: Option<String>,
}

impl UniverseBridge {
    /// Create new universe bridge
    pub fn new(universe_tx: tokio::sync::mpsc::UnboundedSender<UniverseEvent>) -> Self {
        Self { universe_tx }
    }

    /// Emit event to universe
    pub fn emit_event(&self, event_type: &str, data: serde_json::Value) {
        let hash = blake3::hash(serde_json::to_string(&data).unwrap_or_default().as_bytes())
            .to_hex()
            .to_string();

        let event = UniverseEvent {
            hash,
            event_type: event_type.to_string(),
            data,
            timestamp: chrono::Utc::now(),
            parent_hash: None,
        };

        let _ = self.universe_tx.send(event);
    }

    /// Emit device event
    pub fn emit_device_event(&self, device_id: &str, event_type: &str, data: serde_json::Value) {
        let mut full_data = data;
        if let Some(obj) = full_data.as_object_mut() {
            obj.insert("device_id".to_string(), serde_json::Value::String(device_id.to_string()));
        }
        self.emit_event(&format!("device.{}", event_type), full_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_event() {
        let event = TelemetryEvent::new(
            TelemetryEventType::Connected,
            Some("device1".to_string()),
            serde_json::json!({ "ip": "192.168.1.1" }),
        );

        assert_eq!(event.device_id, Some("device1".to_string()));
        assert_eq!(event.severity, "info");
    }

    #[test]
    fn test_telemetry_collector() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let collector = TelemetryCollector::new(tx, 100);

        let event = TelemetryEvent::new(
            TelemetryEventType::Connected,
            Some("device1".to_string()),
            serde_json::json!({}),
        );

        collector.record(event);
        assert_eq!(collector.get_recent_events(10).len(), 1);
    }
}
