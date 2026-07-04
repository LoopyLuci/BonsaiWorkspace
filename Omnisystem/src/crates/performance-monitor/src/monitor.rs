use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct PerformanceMonitor {
    metrics: Arc<DashMap<String, SystemMetrics>>,
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub name: String,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_io: u64,
    pub timestamp: u64,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub fn record_metrics(&self, metrics: SystemMetrics) -> Result<()> {
        self.metrics.insert(metrics.name.clone(), metrics);
        Ok(())
    }

    pub fn get_avg_cpu(&self) -> f32 {
        let total: f32 = self.metrics.iter().map(|m| m.value().cpu_usage).sum();
        if self.metrics.is_empty() { 0.0 } else { total / self.metrics.len() as f32 }
    }

    pub fn get_avg_memory(&self) -> f32 {
        let total: f32 = self.metrics.iter().map(|m| m.value().memory_usage).sum();
        if self.metrics.is_empty() { 0.0 } else { total / self.metrics.len() as f32 }
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor() {
        let monitor = PerformanceMonitor::new();
        let metrics = SystemMetrics {
            name: "system1".to_string(),
            cpu_usage: 45.0,
            memory_usage: 60.0,
            disk_usage: 75.0,
            network_io: 1000,
            timestamp: 1000,
        };
        assert!(monitor.record_metrics(metrics).is_ok());
        assert_eq!(monitor.get_avg_cpu(), 45.0);
    }
}
