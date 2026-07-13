use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TelemetryEvent {
    pub event_type: String,
    pub timestamp: String,
    pub properties: HashMap<String, String>,
}

lazy_static! {
    static ref EVENTS: Mutex<Vec<TelemetryEvent>> = Mutex::new(Vec::new());
}

#[tauri::command]
pub async fn track_event(
    event_type: String,
    properties: HashMap<String, String>,
) -> Result<(), String> {
    let event = TelemetryEvent {
        event_type,
        timestamp: chrono::Utc::now().to_rfc3339(),
        properties,
    };

    match EVENTS.lock() {
        Ok(mut events) => {
            events.push(event);
            Ok(())
        }
        Err(_) => Err("Failed to record telemetry event".to_string()),
    }
}

#[tauri::command]
pub async fn get_telemetry_summary() -> Result<TelemetrySummary, String> {
    match EVENTS.lock() {
        Ok(events) => {
            let mut event_counts: HashMap<String, u32> = HashMap::new();
            for event in events.iter() {
                *event_counts.entry(event.event_type.clone()).or_insert(0) += 1;
            }

            Ok(TelemetrySummary {
                total_events: events.len() as u32,
                events_by_type: event_counts,
            })
        }
        Err(_) => Err("Failed to retrieve telemetry summary".to_string()),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TelemetrySummary {
    pub total_events: u32,
    pub events_by_type: HashMap<String, u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_track_event() {
        let mut props = HashMap::new();
        props.insert("app_id".to_string(), "test-app".to_string());

        let result = track_event("app_launched".to_string(), props).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_telemetry_summary() {
        let mut props = HashMap::new();
        props.insert("test".to_string(), "value".to_string());

        track_event("test_event".to_string(), props.clone()).await.ok();

        let result = get_telemetry_summary().await;
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert!(summary.total_events > 0);
    }
}
