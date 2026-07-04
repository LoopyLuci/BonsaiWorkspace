use crate::{StreamEvent, StreamWindow, WindowType, Aggregation, StreamState, ProcessedResult, StreamError, StreamResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

pub struct StreamProcessor {
    events: Arc<DashMap<Uuid, StreamEvent>>,
    windows: Arc<DashMap<Uuid, StreamWindow>>,
    aggregations: Arc<DashMap<Uuid, Aggregation>>,
    state: Arc<DashMap<String, StreamState>>,
    results: Arc<DashMap<Uuid, ProcessedResult>>,
}

impl StreamProcessor {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            windows: Arc::new(DashMap::new()),
            aggregations: Arc::new(DashMap::new()),
            state: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
        }
    }

    pub async fn emit_event(&self, stream_name: &str, data: HashMap<String, String>) -> StreamResult<StreamEvent> {
        let event = StreamEvent {
            event_id: Uuid::new_v4(),
            stream_name: stream_name.to_string(),
            timestamp: Utc::now(),
            data,
            sequence: self.events.len() as u64 + 1,
        };

        self.events.insert(event.event_id, event.clone());
        Ok(event)
    }

    pub async fn create_window(&self, stream_name: &str, window_type: WindowType, size_ms: u64) -> StreamResult<StreamWindow> {
        let window = StreamWindow {
            window_id: Uuid::new_v4(),
            stream_name: stream_name.to_string(),
            window_type,
            window_size_ms: size_ms,
            start_time: Utc::now(),
            end_time: Utc::now() + chrono::Duration::milliseconds(size_ms as i64),
            event_count: 0,
        };

        self.windows.insert(window.window_id, window.clone());
        Ok(window)
    }

    pub async fn aggregate(&self, window_id: Uuid, agg_type: &str, values: &[f64]) -> StreamResult<Aggregation> {
        if self.windows.get(&window_id).is_none() {
            return Err(StreamError::WindowingFailed);
        }

        let result = match agg_type {
            "sum" => values.iter().sum(),
            "avg" => values.iter().sum::<f64>() / values.len() as f64,
            "max" => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            "min" => values.iter().copied().fold(f64::INFINITY, f64::min),
            "count" => values.len() as f64,
            _ => return Err(StreamError::AggregationFailed),
        };

        let agg = Aggregation {
            agg_id: Uuid::new_v4(),
            window_id,
            agg_type: agg_type.to_string(),
            result,
            computed_at: Utc::now(),
        };

        self.aggregations.insert(agg.agg_id, agg.clone());
        Ok(agg)
    }

    pub async fn put_state(&self, stream_name: &str, key: &str, value: &[u8]) -> StreamResult<StreamState> {
        let state = StreamState {
            state_id: Uuid::new_v4(),
            stream_name: stream_name.to_string(),
            key: key.to_string(),
            state_value: value.to_vec(),
            updated_at: Utc::now(),
            ttl_ms: Some(3600000),
        };

        self.state.insert(key.to_string(), state.clone());
        Ok(state)
    }

    pub async fn get_state(&self, key: &str) -> StreamResult<StreamState> {
        self.state
            .get(key)
            .map(|s| s.value().clone())
            .ok_or(StreamError::StateManagementFailed)
    }

    pub async fn process_stream(&self, stream_name: &str, operation: &str) -> StreamResult<ProcessedResult> {
        let result = ProcessedResult {
            result_id: Uuid::new_v4(),
            source_stream: stream_name.to_string(),
            operation: operation.to_string(),
            output: format!("Processed {} using {}", stream_name, operation),
            processed_at: Utc::now(),
            latency_ms: 5,
        };

        self.results.insert(result.result_id, result.clone());
        Ok(result)
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for StreamProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_emit_event() {
        let processor = StreamProcessor::new();
        let mut data = HashMap::new();
        data.insert("value".to_string(), "100".to_string());

        let event = processor.emit_event("metrics", data).await.unwrap();
        assert_eq!(event.stream_name, "metrics");
        assert_eq!(processor.event_count(), 1);
    }

    #[tokio::test]
    async fn test_create_window() {
        let processor = StreamProcessor::new();
        let window = processor.create_window("orders", WindowType::Tumbling, 60000).await.unwrap();

        assert_eq!(window.window_type, WindowType::Tumbling);
        assert_eq!(window.window_size_ms, 60000);
    }

    #[tokio::test]
    async fn test_aggregate() {
        let processor = StreamProcessor::new();
        let window = processor.create_window("values", WindowType::Sliding, 30000).await.unwrap();

        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let agg = processor.aggregate(window.window_id, "sum", &values).await.unwrap();
        assert_eq!(agg.result, 150.0);
    }

    #[tokio::test]
    async fn test_put_and_get_state() {
        let processor = StreamProcessor::new();
        processor.put_state("stream1", "counter", b"42").await.unwrap();

        let state = processor.get_state("counter").await.unwrap();
        assert_eq!(state.state_value, b"42");
    }
}
