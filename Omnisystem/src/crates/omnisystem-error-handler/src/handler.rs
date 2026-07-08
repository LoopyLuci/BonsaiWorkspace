use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ErrorEvent {
    pub id: String,
    pub message: String,
    pub severity: u8,
}

pub struct ErrorHandler {
    errors: Arc<DashMap<String, ErrorEvent>>,
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self { errors: Arc::new(DashMap::new()) }
    }
    
    pub fn handle_error(&self, message: String, severity: u8) -> String {
        let id = format!("err_{}", self.errors.len());
        let event = ErrorEvent { id: id.clone(), message, severity };
        self.errors.insert(id.clone(), event);
        id
    }
    
    pub fn get_error(&self, id: &str) -> Option<ErrorEvent> {
        self.errors.get(id).map(|e| e.clone())
    }
    
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_handle_error() {
        let handler = ErrorHandler::new();
        let err_id = handler.handle_error("Test error".to_string(), 5);
        assert!(!err_id.is_empty());
    }
    
    #[test]
    fn test_get_error() {
        let handler = ErrorHandler::new();
        let err_id = handler.handle_error("Test error".to_string(), 5);
        assert!(handler.get_error(&err_id).is_some());
    }
}
