use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub value: f64,
}

pub struct MetricsCollector {
    metrics: Arc<DashMap<String, f64>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self { metrics: Arc::new(DashMap::new()) }
    }
    
    pub fn record(&self, name: String, value: f64) {
        self.metrics.insert(name, value);
    }
    
    pub fn get(&self, name: &str) -> Option<f64> {
        self.metrics.get(name).map(|m| *m)
    }
    
    pub fn increment(&self, name: &str, delta: f64) -> bool {
        if let Some(mut metric) = self.metrics.get_mut(name) {
            *metric += delta;
            true
        } else {
            false
        }
    }
    
    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_record() {
        let collector = MetricsCollector::new();
        collector.record("cpu".to_string(), 45.5);
        assert_eq!(collector.get("cpu"), Some(45.5));
    }
    
    #[test]
    fn test_increment() {
        let collector = MetricsCollector::new();
        collector.record("counter".to_string(), 10.0);
        assert!(collector.increment("counter", 5.0));
    }
}
