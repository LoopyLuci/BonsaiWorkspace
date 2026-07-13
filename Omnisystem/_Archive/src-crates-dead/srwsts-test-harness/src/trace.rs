//! Execution trace recording for deterministic replay

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Trace event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// System call
    Syscall,
    /// Memory access
    MemoryAccess,
    /// Context switch
    ContextSwitch,
    /// Interrupt
    Interrupt,
    /// I/O operation
    IoOperation,
    /// Network operation
    NetworkOperation,
    /// Synchronization primitive
    Sync,
    /// Custom event
    Custom,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syscall => write!(f, "Syscall"),
            Self::MemoryAccess => write!(f, "MemoryAccess"),
            Self::ContextSwitch => write!(f, "ContextSwitch"),
            Self::Interrupt => write!(f, "Interrupt"),
            Self::IoOperation => write!(f, "IoOperation"),
            Self::NetworkOperation => write!(f, "NetworkOperation"),
            Self::Sync => write!(f, "Sync"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

/// Trace event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Event ID
    pub id: u64,
    /// Event type
    pub event_type: EventType,
    /// Timestamp (microseconds since start)
    pub timestamp_us: u64,
    /// Thread/Process ID
    pub tid: u32,
    /// Core ID (if applicable)
    pub core_id: Option<usize>,
    /// Event description
    pub description: String,
    /// Additional data
    pub data: HashMap<String, String>,
}

impl TraceEvent {
    /// Create a new trace event
    pub fn new(
        id: u64,
        event_type: EventType,
        timestamp_us: u64,
        tid: u32,
        description: String,
    ) -> Self {
        Self {
            id,
            event_type,
            timestamp_us,
            tid,
            core_id: None,
            description,
            data: HashMap::new(),
        }
    }

    /// Set core ID
    pub fn with_core(mut self, core_id: usize) -> Self {
        self.core_id = Some(core_id);
        self
    }

    /// Add data field
    pub fn with_data(mut self, key: String, value: String) -> Self {
        self.data.insert(key, value);
        self
    }
}

/// Execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    /// Trace ID
    pub id: Uuid,
    /// Test name
    pub test_name: String,
    /// Events in chronological order
    pub events: Vec<TraceEvent>,
    /// Start timestamp
    pub start_time: DateTime<Utc>,
    /// End timestamp
    pub end_time: Option<DateTime<Utc>>,
    /// Total duration (microseconds)
    pub duration_us: u64,
    /// Number of events
    pub event_count: u64,
}

impl ExecutionTrace {
    /// Create a new execution trace
    pub fn new(test_name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            test_name,
            events: Vec::new(),
            start_time: Utc::now(),
            end_time: None,
            duration_us: 0,
            event_count: 0,
        }
    }

    /// Add an event to the trace
    pub fn add_event(&mut self, event: TraceEvent) {
        self.events.push(event);
        self.event_count += 1;
    }

    /// Mark trace as completed
    pub fn complete(&mut self) {
        self.end_time = Some(Utc::now());
        if let Some(last_event) = self.events.last() {
            self.duration_us = last_event.timestamp_us;
        }
    }

    /// Get events by type
    pub fn events_by_type(&self, event_type: EventType) -> Vec<&TraceEvent> {
        self.events
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect()
    }

    /// Get events for a specific thread
    pub fn events_for_thread(&self, tid: u32) -> Vec<&TraceEvent> {
        self.events.iter().filter(|e| e.tid == tid).collect()
    }

    /// Find event by ID
    pub fn find_event(&self, event_id: u64) -> Option<&TraceEvent> {
        self.events.iter().find(|e| e.id == event_id)
    }

    /// Get timeline of events
    pub fn timeline(&self) -> impl Iterator<Item = &TraceEvent> {
        self.events.iter()
    }
}

/// Trace recorder
pub struct TraceRecorder {
    /// Current trace
    current_trace: Option<ExecutionTrace>,
    /// Completed traces
    completed_traces: Vec<ExecutionTrace>,
    /// Event ID counter
    event_counter: u64,
}

impl TraceRecorder {
    /// Create a new trace recorder
    pub fn new() -> Self {
        Self {
            current_trace: None,
            completed_traces: Vec::new(),
            event_counter: 0,
        }
    }

    /// Start recording a new trace
    pub fn start_trace(&mut self, test_name: String) {
        self.current_trace = Some(ExecutionTrace::new(test_name));
        self.event_counter = 0;
    }

    /// Record an event
    pub fn record_event(
        &mut self,
        event_type: EventType,
        timestamp_us: u64,
        tid: u32,
        description: String,
    ) -> u64 {
        if let Some(trace) = &mut self.current_trace {
            let event_id = self.event_counter;
            self.event_counter += 1;

            let event = TraceEvent::new(event_id, event_type, timestamp_us, tid, description);
            trace.add_event(event);
            event_id
        } else {
            0
        }
    }

    /// Complete the current trace
    pub fn complete_trace(&mut self) -> Option<ExecutionTrace> {
        if let Some(mut trace) = self.current_trace.take() {
            trace.complete();
            self.completed_traces.push(trace.clone());
            Some(trace)
        } else {
            None
        }
    }

    /// Get current trace (immutable)
    pub fn current_trace(&self) -> Option<&ExecutionTrace> {
        self.current_trace.as_ref()
    }

    /// Get completed traces
    pub fn completed_traces(&self) -> &[ExecutionTrace] {
        &self.completed_traces
    }

    /// Save trace to bytes (serialized format)
    pub fn serialize_trace(&self, trace: &ExecutionTrace) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(trace)
    }

    /// Load trace from bytes
    pub fn deserialize_trace(&mut self, data: &[u8]) -> Result<ExecutionTrace, serde_json::Error> {
        serde_json::from_slice(data)
    }

    /// Reset recorder
    pub fn reset(&mut self) {
        self.current_trace = None;
        self.completed_traces.clear();
        self.event_counter = 0;
    }

    /// Get trace statistics
    pub fn statistics(&self) -> TraceStatistics {
        let total_events: u64 = self.completed_traces.iter().map(|t| t.event_count).sum();
        let total_duration_us: u64 = self.completed_traces.iter().map(|t| t.duration_us).sum();

        TraceStatistics {
            total_traces: self.completed_traces.len(),
            total_events,
            total_duration_us,
        }
    }
}

impl Default for TraceRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Trace statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStatistics {
    /// Total number of traces recorded
    pub total_traces: usize,
    /// Total events across all traces
    pub total_events: u64,
    /// Total duration across all traces (microseconds)
    pub total_duration_us: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_event_creation() {
        let event = TraceEvent::new(1, EventType::Syscall, 1000, 1234, "test syscall".to_string());
        assert_eq!(event.id, 1);
        assert_eq!(event.event_type, EventType::Syscall);
        assert_eq!(event.tid, 1234);
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::Syscall.to_string(), "Syscall");
        assert_eq!(EventType::MemoryAccess.to_string(), "MemoryAccess");
    }

    #[test]
    fn test_execution_trace_creation() {
        let trace = ExecutionTrace::new("test".to_string());
        assert_eq!(trace.test_name, "test");
        assert_eq!(trace.event_count, 0);
    }

    #[test]
    fn test_execution_trace_add_event() {
        let mut trace = ExecutionTrace::new("test".to_string());
        let event = TraceEvent::new(1, EventType::Syscall, 1000, 1, "test".to_string());

        trace.add_event(event);
        assert_eq!(trace.event_count, 1);
        assert_eq!(trace.events.len(), 1);
    }

    #[test]
    fn test_trace_events_by_type() {
        let mut trace = ExecutionTrace::new("test".to_string());

        let event1 = TraceEvent::new(1, EventType::Syscall, 1000, 1, "syscall".to_string());
        let event2 = TraceEvent::new(2, EventType::MemoryAccess, 2000, 1, "memory".to_string());
        let event3 = TraceEvent::new(3, EventType::Syscall, 3000, 1, "syscall".to_string());

        trace.add_event(event1);
        trace.add_event(event2);
        trace.add_event(event3);

        let syscalls = trace.events_by_type(EventType::Syscall);
        assert_eq!(syscalls.len(), 2);
    }

    #[test]
    fn test_trace_events_for_thread() {
        let mut trace = ExecutionTrace::new("test".to_string());

        let event1 = TraceEvent::new(1, EventType::Syscall, 1000, 1, "e1".to_string());
        let event2 = TraceEvent::new(2, EventType::Syscall, 2000, 2, "e2".to_string());

        trace.add_event(event1);
        trace.add_event(event2);

        let tid1_events = trace.events_for_thread(1);
        assert_eq!(tid1_events.len(), 1);
    }

    #[test]
    fn test_trace_recorder_basic() {
        let mut recorder = TraceRecorder::new();

        recorder.start_trace("test".to_string());
        recorder.record_event(
            EventType::Syscall,
            1000,
            1,
            "syscall".to_string(),
        );

        let completed = recorder.complete_trace();
        assert!(completed.is_some());
        assert_eq!(recorder.completed_traces.len(), 1);
    }

    #[test]
    fn test_trace_recorder_statistics() {
        let mut recorder = TraceRecorder::new();

        recorder.start_trace("test1".to_string());
        recorder.record_event(EventType::Syscall, 1000, 1, "e1".to_string());
        recorder.record_event(EventType::Syscall, 2000, 1, "e2".to_string());
        recorder.complete_trace();

        let stats = recorder.statistics();
        assert_eq!(stats.total_traces, 1);
        assert_eq!(stats.total_events, 2);
    }

    #[test]
    fn test_trace_serialization() {
        let recorder = TraceRecorder::new();
        let trace = ExecutionTrace::new("test".to_string());

        let serialized = recorder.serialize_trace(&trace);
        assert!(serialized.is_ok());

        let bytes = serialized.unwrap();
        let deserialized = serde_json::from_slice::<ExecutionTrace>(&bytes);
        assert!(deserialized.is_ok());
    }
}
