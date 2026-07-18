use crate::{StreamEvent, StreamWindow, WindowType, Aggregation, StreamState, ProcessedResult, StreamError, StreamResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Instant;

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
        let mut window = self
            .windows
            .get_mut(&window_id)
            .ok_or(StreamError::WindowingFailed)?;

        if values.is_empty() && agg_type != "count" {
            return Err(StreamError::AggregationFailed);
        }

        let result = match agg_type {
            "sum" => values.iter().sum(),
            "avg" => values.iter().sum::<f64>() / values.len() as f64,
            "max" => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            "min" => values.iter().copied().fold(f64::INFINITY, f64::min),
            "count" => values.len() as f64,
            _ => return Err(StreamError::AggregationFailed),
        };

        // Track how many events have flowed through this window.
        window.event_count += values.len() as u32;
        drop(window);

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

    /// Run `operation` over the events currently buffered for `stream_name`
    /// and record the real wall-clock latency of doing so.
    pub async fn process_stream(&self, stream_name: &str, operation: &str) -> StreamResult<ProcessedResult> {
        let started = Instant::now();

        let matching: Vec<StreamEvent> = self
            .events
            .iter()
            .filter(|e| e.value().stream_name == stream_name)
            .map(|e| e.value().clone())
            .collect();

        let output = match operation {
            "count" => matching.len().to_string(),
            "latest" => matching
                .iter()
                .max_by_key(|e| e.sequence)
                .map(|e| format!("{:?}", e.data))
                .unwrap_or_else(|| "no events".to_string()),
            "sequences" => {
                let mut seqs: Vec<u64> = matching.iter().map(|e| e.sequence).collect();
                seqs.sort_unstable();
                format!("{:?}", seqs)
            }
            other => return Err(StreamError::Other(format!("unsupported operation: {other}"))),
        };

        let latency_ms = started.elapsed().as_millis() as u64;

        let result = ProcessedResult {
            result_id: Uuid::new_v4(),
            source_stream: stream_name.to_string(),
            operation: operation.to_string(),
            output,
            processed_at: Utc::now(),
            latency_ms,
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
    async fn test_aggregate_tracks_event_count_and_rejects_unknown_window() {
        let processor = StreamProcessor::new();
        let window = processor.create_window("values", WindowType::Tumbling, 1000).await.unwrap();

        processor.aggregate(window.window_id, "avg", &[1.0, 2.0, 3.0]).await.unwrap();
        processor.aggregate(window.window_id, "max", &[4.0, 5.0]).await.unwrap();

        let unknown = processor.aggregate(Uuid::new_v4(), "sum", &[1.0]).await;
        assert!(unknown.is_err());

        let bad_type = processor.aggregate(window.window_id, "median", &[1.0]).await;
        assert!(bad_type.is_err());
    }

    #[tokio::test]
    async fn test_put_and_get_state() {
        let processor = StreamProcessor::new();
        processor.put_state("stream1", "counter", b"42").await.unwrap();

        let state = processor.get_state("counter").await.unwrap();
        assert_eq!(state.state_value, b"42");
    }

    #[tokio::test]
    async fn test_get_state_missing_key_fails() {
        let processor = StreamProcessor::new();
        let result = processor.get_state("missing").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_stream_count_and_sequences() {
        let processor = StreamProcessor::new();
        let mut d1 = HashMap::new();
        d1.insert("v".to_string(), "1".to_string());
        let mut d2 = HashMap::new();
        d2.insert("v".to_string(), "2".to_string());

        processor.emit_event("orders", d1).await.unwrap();
        processor.emit_event("orders", d2).await.unwrap();
        processor.emit_event("other", HashMap::new()).await.unwrap();

        let count_result = processor.process_stream("orders", "count").await.unwrap();
        assert_eq!(count_result.output, "2");

        let seq_result = processor.process_stream("orders", "sequences").await.unwrap();
        assert_eq!(seq_result.output, "[1, 2]");

        let unsupported = processor.process_stream("orders", "bogus").await;
        assert!(unsupported.is_err());
    }
}
