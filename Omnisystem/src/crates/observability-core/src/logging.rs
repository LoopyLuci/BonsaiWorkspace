use crate::{LogEntry, LogLevel, ObservabilityError, ObservabilityResult, TraceId};
use dashmap::DashMap;
use std::sync::Arc;

pub struct LogCollector {
    logs: Arc<DashMap<String, Vec<LogEntry>>>,
    batch_size: usize,
}

impl LogCollector {
    pub fn new(batch_size: usize) -> Self {
        Self {
            logs: Arc::new(DashMap::new()),
            batch_size,
        }
    }

    pub async fn write_log(&self, entry: &LogEntry) -> ObservabilityResult<()> {
        let trace_key = entry.trace_id.as_ref().map(|t| t.0.clone()).unwrap_or_else(|| "no-trace".to_string());

        self.logs
            .entry(trace_key)
            .or_insert_with(Vec::new)
            .push(entry.clone());

        Ok(())
    }

    pub async fn write_batch(&self, entries: Vec<LogEntry>) -> ObservabilityResult<()> {
        if entries.len() > self.batch_size * 10 {
            return Err(ObservabilityError::LogCollectionFailed("Batch too large".to_string()));
        }

        for entry in entries {
            self.write_log(&entry).await?;
        }
        Ok(())
    }

    pub async fn query_logs(
        &self,
        trace_id: Option<&TraceId>,
        level: Option<LogLevel>,
        limit: usize,
    ) -> ObservabilityResult<Vec<LogEntry>> {
        let mut results = Vec::new();

        if let Some(trace_id) = trace_id {
            if let Some(entries) = self.logs.get(&trace_id.0) {
                for entry in entries.iter() {
                    if level.is_none() || entry.level == *level.as_ref().unwrap() {
                        results.push(entry.clone());
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
        } else {
            for entry_ref in self.logs.iter() {
                for entry in entry_ref.value().iter() {
                    if level.is_none() || entry.level == *level.as_ref().unwrap() {
                        results.push(entry.clone());
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
                if results.len() >= limit {
                    break;
                }
            }
        }

        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(results)
    }

    pub async fn get_logs_for_trace(&self, trace_id: &TraceId) -> ObservabilityResult<Vec<LogEntry>> {
        self.logs
            .get(&trace_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| ObservabilityError::TraceNotFound(trace_id.0.clone()))
    }

    pub async fn flush(&self) -> ObservabilityResult<()> {
        self.logs.clear();
        Ok(())
    }

    pub fn log_count(&self) -> usize {
        self.logs.iter().map(|entry| entry.value().len()).sum()
    }

    pub fn trace_count(&self) -> usize {
        self.logs.len()
    }
}

impl Default for LogCollector {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_log_entry(level: LogLevel, message: &str, trace_id: Option<TraceId>) -> LogEntry {
        LogEntry {
            timestamp: Utc::now(),
            level,
            message: message.to_string(),
            service: "test-service".to_string(),
            trace_id,
            span_id: None,
            correlation_id: None,
            fields: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_write_log() {
        let collector = LogCollector::new(100);
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let entry = create_log_entry(LogLevel::Info, "test message", Some(trace_id));

        collector.write_log(&entry).await.unwrap();
        assert_eq!(collector.log_count(), 1);
    }

    #[tokio::test]
    async fn test_write_batch() {
        let collector = LogCollector::new(100);
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let entries = vec![
            create_log_entry(LogLevel::Info, "msg1", Some(trace_id.clone())),
            create_log_entry(LogLevel::Warn, "msg2", Some(trace_id.clone())),
            create_log_entry(LogLevel::Error, "msg3", Some(trace_id.clone())),
        ];

        collector.write_batch(entries).await.unwrap();
        assert_eq!(collector.log_count(), 3);
    }

    #[tokio::test]
    async fn test_query_logs_by_level() {
        let collector = LogCollector::new(100);
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());

        collector.write_log(&create_log_entry(LogLevel::Info, "info", Some(trace_id.clone()))).await.unwrap();
        collector.write_log(&create_log_entry(LogLevel::Error, "error", Some(trace_id.clone()))).await.unwrap();

        let results = collector.query_logs(Some(&trace_id), Some(LogLevel::Error), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].level, LogLevel::Error);
    }

    #[tokio::test]
    async fn test_query_logs_limit() {
        let collector = LogCollector::new(100);
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());

        for i in 0..10 {
            collector
                .write_log(&create_log_entry(LogLevel::Info, &format!("msg{}", i), Some(trace_id.clone())))
                .await
                .unwrap();
        }

        let results = collector.query_logs(Some(&trace_id), None, 5).await.unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_get_logs_for_trace() {
        let collector = LogCollector::new(100);
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());

        collector.write_log(&create_log_entry(LogLevel::Info, "test", Some(trace_id.clone()))).await.unwrap();

        let logs = collector.get_logs_for_trace(&trace_id).await.unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[tokio::test]
    async fn test_flush() {
        let collector = LogCollector::new(100);
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());

        collector.write_log(&create_log_entry(LogLevel::Info, "test", Some(trace_id))).await.unwrap();
        assert!(collector.log_count() > 0);

        collector.flush().await.unwrap();
        assert_eq!(collector.log_count(), 0);
    }

    #[tokio::test]
    async fn test_batch_too_large() {
        let collector = LogCollector::new(100);
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());

        let mut entries = Vec::new();
        for i in 0..1001 {
            entries.push(create_log_entry(LogLevel::Info, &format!("msg{}", i), Some(trace_id.clone())));
        }

        let result = collector.write_batch(entries).await;
        assert!(result.is_err());
    }
}
