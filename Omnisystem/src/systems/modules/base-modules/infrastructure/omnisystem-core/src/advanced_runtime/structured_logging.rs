//! Structured Logging System with Distributed Tracing
//!
//! Provides JSON logging with:
//! - Structured log events with context
//! - Distributed tracing with correlation IDs
//! - Dynamic log level filtering
//! - Performance tracking with spans
//! - Ring buffer for bounded memory
//! - Async non-blocking logging

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use super::LogLevel;

/// Log context for distributed tracing
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct LogContext {
    pub actor_id: Option<String>,
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub duration_us: Option<u64>,
    pub service: Option<String>,
    pub version: Option<String>,
}

/// Structured log event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: String,
    pub message: String,
    pub context: LogContext,
    pub timestamp: u64, // Unix timestamp in milliseconds
    pub event_id: String,
    pub metadata: std::collections::HashMap<String, String>,
}

impl LogEvent {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        Self {
            level: format!("{:?}", level),
            message: message.into(),
            context: LogContext::default(),
            timestamp: now.as_millis() as u64,
            event_id: uuid::Uuid::new_v4().to_string(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_context(mut self, context: LogContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Performance span for timing measurements
#[derive(Clone, Debug)]
pub struct Span {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub tags: std::collections::HashMap<String, String>,
}

impl Span {
    pub fn new(name: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            name: name.into(),
            start_time: now.as_millis() as u64,
            end_time: None,
            tags: std::collections::HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn end(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        self.end_time = Some(now.as_millis() as u64);
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.end_time.map(|end| end - self.start_time)
    }

    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Structured logger with JSON output
pub struct StructuredLogger {
    level: Arc<AtomicUsize>,
    events: Arc<Mutex<VecDeque<LogEvent>>>,
    max_events: usize,
    spans: Arc<Mutex<Vec<Span>>>,
    enabled: Arc<AtomicBool>,
}

impl StructuredLogger {
    pub fn new(level: LogLevel) -> Self {
        Self {
            level: Arc::new(AtomicUsize::new(level.to_level_value())),
            events: Arc::new(Mutex::new(VecDeque::with_capacity(10000))),
            max_events: 10000,
            spans: Arc::new(Mutex::new(Vec::new())),
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn log(&self, event: LogEvent) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }

        let event_level = Self::parse_level(&event.level);
        let current_level = self.level.load(Ordering::Relaxed);

        if event_level < current_level {
            return; // Skip events below log level
        }

        // Print to stdout (production would send to log aggregator)
        println!("{}", serde_json::to_string(&event).unwrap_or_else(|_| event.message.clone()));

        // Store in ring buffer
        let mut events = self.events.lock();
        if events.len() >= self.max_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub fn start_span(&self, span: Span) -> String {
        let span_id = span.id.clone();
        let mut spans = self.spans.lock();
        spans.push(span);
        span_id
    }

    pub fn end_span(&self, span_id: &str) -> Option<Span> {
        let mut spans = self.spans.lock();
        if let Some(pos) = spans.iter().position(|s| s.id == span_id) {
            let mut span = spans.remove(pos);
            span.end();
            Some(span)
        } else {
            None
        }
    }

    pub fn set_log_level(&self, level: LogLevel) {
        self.level.store(level.to_level_value(), Ordering::Release);
    }

    pub fn get_log_level(&self) -> LogLevel {
        let level_value = self.level.load(Ordering::Relaxed);
        LogLevel::from_level_value(level_value)
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Release);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn get_events(&self) -> Vec<LogEvent> {
        let events = self.events.lock();
        events.iter().cloned().collect()
    }

    pub fn get_recent_events(&self, count: usize) -> Vec<LogEvent> {
        let events = self.events.lock();
        events
            .iter()
            .rev()
            .take(count)
            .rev()
            .cloned()
            .collect()
    }

    pub fn clear_events(&self) {
        self.events.lock().clear();
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().len()
    }

    fn parse_level(level: &str) -> usize {
        match level {
            "Trace" => 0,
            "Debug" => 1,
            "Info" => 2,
            "Warn" => 3,
            "Error" => 4,
            "Critical" => 5,
            _ => 2, // Default to Info
        }
    }
}

impl Clone for StructuredLogger {
    fn clone(&self) -> Self {
        Self {
            level: self.level.clone(),
            events: self.events.clone(),
            max_events: self.max_events,
            spans: self.spans.clone(),
            enabled: self.enabled.clone(),
        }
    }
}

/// Performance logger for measurements
pub struct PerfLogger {
    logger: Arc<StructuredLogger>,
}

impl PerfLogger {
    pub fn new(logger: Arc<StructuredLogger>) -> Self {
        Self { logger }
    }

    pub fn measure<F, R>(&self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = SystemTime::now();
        let result = f();
        let duration = start.elapsed().unwrap_or_default();

        let mut context = LogContext::default();
        context.duration_us = Some(duration.as_micros() as u64);

        let event = LogEvent::new(LogLevel::Debug, format!("Performance: {} took {:?}", name, duration))
            .with_context(context);

        self.logger.log(event);
        result
    }

    pub async fn measure_async<F, Fut, R>(&self, name: &str, f: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = SystemTime::now();
        let result = f().await;
        let duration = start.elapsed().unwrap_or_default();

        let mut context = LogContext::default();
        context.duration_us = Some(duration.as_micros() as u64);

        let event = LogEvent::new(LogLevel::Debug, format!("Performance: {} took {:?}", name, duration))
            .with_context(context);

        self.logger.log(event);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_event_creation() {
        let event = LogEvent::new(LogLevel::Info, "Test message");
        assert_eq!(event.message, "Test message");
        assert!(!event.event_id.is_empty());
    }

    #[test]
    fn test_logger_creation() {
        let logger = StructuredLogger::new(LogLevel::Info);
        assert!(logger.is_enabled());
    }

    #[test]
    fn test_log_event() {
        let logger = StructuredLogger::new(LogLevel::Info);
        let event = LogEvent::new(LogLevel::Info, "Test");
        logger.log(event);
        assert_eq!(logger.event_count(), 1);
    }

    #[test]
    fn test_log_level_filtering() {
        let logger = StructuredLogger::new(LogLevel::Warn);
        logger.log(LogEvent::new(LogLevel::Debug, "Debug"));
        logger.log(LogEvent::new(LogLevel::Info, "Info"));
        logger.log(LogEvent::new(LogLevel::Warn, "Warn"));
        logger.log(LogEvent::new(LogLevel::Error, "Error"));

        let events = logger.get_events();
        assert_eq!(events.len(), 2); // Only Warn and Error
    }

    #[test]
    fn test_span_creation() {
        let span = Span::new("test_operation");
        assert_eq!(span.name, "test_operation");
        assert!(span.end_time.is_none());
    }

    #[test]
    fn test_span_timing() {
        let mut span = Span::new("test_operation");
        span.end();
        assert!(span.end_time.is_some());
        assert!(span.duration_ms().is_some());
    }

    #[test]
    fn test_logger_span_management() {
        let logger = StructuredLogger::new(LogLevel::Info);
        let span = Span::new("operation");
        let span_id = logger.start_span(span);

        assert!(!span_id.is_empty());

        let ended_span = logger.end_span(&span_id);
        assert!(ended_span.is_some());
    }

    #[test]
    fn test_log_level_change() {
        let logger = StructuredLogger::new(LogLevel::Info);
        logger.set_log_level(LogLevel::Error);

        let level = logger.get_log_level();
        assert_eq!(level, LogLevel::Error);
    }

    #[test]
    fn test_logger_disable() {
        let logger = StructuredLogger::new(LogLevel::Info);
        logger.disable();
        assert!(!logger.is_enabled());

        logger.log(LogEvent::new(LogLevel::Info, "Should not be logged"));
        assert_eq!(logger.event_count(), 0);
    }

    #[test]
    fn test_recent_events() {
        let logger = StructuredLogger::new(LogLevel::Debug);

        for i in 0..20 {
            logger.log(LogEvent::new(LogLevel::Info, format!("Event {}", i)));
        }

        let recent = logger.get_recent_events(5);
        assert_eq!(recent.len(), 5);
    }
}
