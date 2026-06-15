//! Omnisystem Observability (OOBS)
//!
//! Integrated tracing, metrics, and observability without external dependencies.
//! Provides structured logging, metrics collection, and distributed tracing.

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Instant;

/// Log level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Structured log event
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub level: LogLevel,
    pub message: String,
    pub fields: HashMap<String, String>,
    pub timestamp: u64,
}

/// Global tracer for distributed tracing
pub struct Tracer {
    spans: Arc<Mutex<Vec<Span>>>,
}

/// Distributed trace span
#[derive(Debug, Clone)]
pub struct Span {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub start: Instant,
    pub end: Option<Instant>,
    pub fields: HashMap<String, String>,
}

impl Span {
    /// Create a new span
    pub fn new(id: u64, name: String) -> Self {
        Span {
            id,
            parent_id: None,
            name,
            start: Instant::now(),
            end: None,
            fields: HashMap::new(),
        }
    }

    /// Finish the span
    pub fn finish(&mut self) {
        self.end = Some(Instant::now());
    }

    /// Get duration in microseconds
    pub fn duration_micros(&self) -> Option<u64> {
        self.end.map(|e| {
            let duration = e.duration_since(self.start);
            duration.as_micros() as u64
        })
    }

    /// Add field to span
    pub fn add_field(&mut self, key: String, value: String) {
        self.fields.insert(key, value);
    }
}

impl Tracer {
    /// Create new tracer
    pub fn new() -> Self {
        Tracer {
            spans: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a new span
    pub fn span(&self, id: u64, name: String) -> Span {
        Span::new(id, name)
    }

    /// Record a span
    pub fn record_span(&self, span: Span) {
        if let Ok(mut spans) = self.spans.lock() {
            spans.push(span);
        }
    }

    /// Get all recorded spans
    pub fn spans(&self) -> Vec<Span> {
        self.spans.lock().ok().map(|s| s.clone()).unwrap_or_default()
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics collector
pub struct MetricsCollector {
    counters: Arc<Mutex<HashMap<String, u64>>>,
    gauges: Arc<Mutex<HashMap<String, f64>>>,
    histograms: Arc<Mutex<HashMap<String, Vec<u64>>>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        MetricsCollector {
            counters: Arc::new(Mutex::new(HashMap::new())),
            gauges: Arc::new(Mutex::new(HashMap::new())),
            histograms: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Increment counter
    pub fn increment_counter(&self, name: &str, value: u64) {
        if let Ok(mut counters) = self.counters.lock() {
            *counters.entry(name.to_string()).or_insert(0) += value;
        }
    }

    /// Set gauge value
    pub fn set_gauge(&self, name: &str, value: f64) {
        if let Ok(mut gauges) = self.gauges.lock() {
            gauges.insert(name.to_string(), value);
        }
    }

    /// Record histogram value
    pub fn record_histogram(&self, name: &str, value: u64) {
        if let Ok(mut histograms) = self.histograms.lock() {
            histograms
                .entry(name.to_string())
                .or_insert_with(Vec::new)
                .push(value);
        }
    }

    /// Get counter value
    pub fn get_counter(&self, name: &str) -> Option<u64> {
        self.counters.lock().ok().and_then(|c| c.get(name).copied())
    }

    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.lock().ok().and_then(|g| g.get(name).copied())
    }

    /// Get histogram percentile
    pub fn get_histogram_percentile(&self, name: &str, percentile: f64) -> Option<u64> {
        self.histograms.lock().ok().and_then(|h| {
            h.get(name).and_then(|values| {
                if values.is_empty() {
                    return None;
                }
                let mut sorted = values.clone();
                sorted.sort_unstable();
                let idx = ((percentile / 100.0) * sorted.len() as f64) as usize;
                Some(sorted[idx.min(sorted.len() - 1)])
            })
        })
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Logger with structured output
pub struct Logger {
    level: LogLevel,
    events: Arc<Mutex<Vec<LogEvent>>>,
}

impl Logger {
    /// Create new logger
    pub fn new(level: LogLevel) -> Self {
        Logger {
            level,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Log a message
    pub fn log(&self, level: LogLevel, message: impl Into<String>) {
        if level >= self.level {
            let event = LogEvent {
                level,
                message: message.into(),
                fields: HashMap::new(),
                timestamp: current_millis(),
            };
            if let Ok(mut events) = self.events.lock() {
                events.push(event);
            }
        }
    }

    /// Log with fields
    pub fn log_with_fields(&self, level: LogLevel, message: impl Into<String>, fields: HashMap<String, String>) {
        if level >= self.level {
            let event = LogEvent {
                level,
                message: message.into(),
                fields,
                timestamp: current_millis(),
            };
            if let Ok(mut events) = self.events.lock() {
                events.push(event);
            }
        }
    }

    /// Get all events
    pub fn events(&self) -> Vec<LogEvent> {
        self.events.lock().ok().map(|e| e.clone()).unwrap_or_default()
    }

    /// Clear events
    pub fn clear(&self) {
        if let Ok(mut events) = self.events.lock() {
            events.clear();
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

fn current_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_basic() {
        let logger = Logger::new(LogLevel::Debug);
        logger.log(LogLevel::Info, "test message");
        let events = logger.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, LogLevel::Info);
    }

    #[test]
    fn test_metrics_counter() {
        let metrics = MetricsCollector::new();
        metrics.increment_counter("requests", 5);
        metrics.increment_counter("requests", 3);
        assert_eq!(metrics.get_counter("requests"), Some(8));
    }

    #[test]
    fn test_tracer_span() {
        let tracer = Tracer::new();
        let mut span = tracer.span(1, "test".to_string());
        span.add_field("key".to_string(), "value".to_string());
        span.finish();
        tracer.record_span(span);
        let spans = tracer.spans();
        assert_eq!(spans.len(), 1);
        assert!(spans[0].duration_micros().is_some());
    }

    #[test]
    fn test_metrics_gauge() {
        let metrics = MetricsCollector::new();
        metrics.set_gauge("temperature", 23.5);
        assert_eq!(metrics.get_gauge("temperature"), Some(23.5));
    }

    #[test]
    fn test_metrics_histogram() {
        let metrics = MetricsCollector::new();
        metrics.record_histogram("latency", 100);
        metrics.record_histogram("latency", 200);
        metrics.record_histogram("latency", 300);
        assert_eq!(metrics.get_histogram_percentile("latency", 50.0), Some(200));
    }
}
