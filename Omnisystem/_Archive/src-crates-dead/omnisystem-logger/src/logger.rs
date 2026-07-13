use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub id: String,
    pub message: String,
    pub level: String,
}

pub struct Logger {
    logs: Arc<DashMap<String, LogEntry>>,
}

impl Logger {
    pub fn new() -> Self {
        Self { logs: Arc::new(DashMap::new()) }
    }
    
    pub fn log(&self, message: String, level: String) -> String {
        let id = format!("log_{}", self.logs.len());
        let entry = LogEntry { id: id.clone(), message, level };
        self.logs.insert(id.clone(), entry);
        id
    }
    
    pub fn get_log(&self, id: &str) -> Option<LogEntry> {
        self.logs.get(id).map(|l| l.clone())
    }
    
    pub fn log_count(&self) -> usize {
        self.logs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log() {
        let logger = Logger::new();
        let log_id = logger.log("Test log".to_string(), "INFO".to_string());
        assert!(!log_id.is_empty());
    }
    
    #[test]
    fn test_get_log() {
        let logger = Logger::new();
        let log_id = logger.log("Test log".to_string(), "INFO".to_string());
        assert!(logger.get_log(&log_id).is_some());
    }
}
