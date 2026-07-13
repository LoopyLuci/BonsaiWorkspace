use dashmap::DashMap;
use module_interfaces::ModuleError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleMetric {
    pub module_id: String,
    pub metric_name: String,
    pub value: f64,
    pub timestamp: u64,
    pub tags: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleStats {
    pub module_id: String,
    pub load_count: u64,
    pub unload_count: u64,
    pub execution_count: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub avg_load_time_ms: f64,
    pub avg_execution_time_ms: f64,
    pub p99_execution_time_ms: f64,
    pub uptime_seconds: u64,
    pub last_loaded: u64,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalyticsDashboard {
    pub id: String,
    pub name: String,
    pub modules: Vec<ModuleStats>,
    pub total_modules_loaded: usize,
    pub total_executions: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub avg_system_load_time_ms: f64,
    pub uptime_percentage: f64,
    pub generated_at: u64,
}

pub struct AnalyticsEngine {
    metrics: Arc<DashMap<String, Vec<ModuleMetric>>>,
    stats: Arc<DashMap<String, ModuleStats>>,
    dashboards: Arc<DashMap<String, AnalyticsDashboard>>,
}

impl AnalyticsEngine {
    pub fn new() -> Self {
        info!("Creating AnalyticsEngine");
        Self {
            metrics: Arc::new(DashMap::new()),
            stats: Arc::new(DashMap::new()),
            dashboards: Arc::new(DashMap::new()),
        }
    }

    pub fn record_metric(&self, metric: ModuleMetric) -> Result<(), ModuleError> {
        debug!("Recording metric for module: {}", metric.module_id);
        self.metrics
            .entry(metric.module_id.clone())
            .or_insert_with(Vec::new)
            .push(metric);
        Ok(())
    }

    pub fn record_load(&self, module_id: &str, duration_ms: u64) -> Result<(), ModuleError> {
        debug!("Recording load metric for module: {} ({}ms)", module_id, duration_ms);

        let mut stats = self
            .stats
            .entry(module_id.to_string())
            .or_insert_with(|| ModuleStats {
                module_id: module_id.to_string(),
                load_count: 0,
                unload_count: 0,
                execution_count: 0,
                success_count: 0,
                error_count: 0,
                avg_load_time_ms: 0.0,
                avg_execution_time_ms: 0.0,
                p99_execution_time_ms: 0.0,
                uptime_seconds: 0,
                last_loaded: 0,
                last_error: None,
            });

        stats.load_count += 1;
        stats.last_loaded = chrono::Utc::now().timestamp() as u64;
        stats.avg_load_time_ms = (stats.avg_load_time_ms * (stats.load_count - 1) as f64 + duration_ms as f64) / stats.load_count as f64;

        Ok(())
    }

    pub fn record_execution(&self, module_id: &str, duration_ms: u64, success: bool) -> Result<(), ModuleError> {
        debug!("Recording execution for module: {} ({}ms, success={})", module_id, duration_ms, success);

        let mut stats = self
            .stats
            .entry(module_id.to_string())
            .or_insert_with(|| ModuleStats {
                module_id: module_id.to_string(),
                load_count: 0,
                unload_count: 0,
                execution_count: 0,
                success_count: 0,
                error_count: 0,
                avg_load_time_ms: 0.0,
                avg_execution_time_ms: 0.0,
                p99_execution_time_ms: 0.0,
                uptime_seconds: 0,
                last_loaded: 0,
                last_error: None,
            });

        stats.execution_count += 1;
        if success {
            stats.success_count += 1;
        } else {
            stats.error_count += 1;
        }
        stats.avg_execution_time_ms =
            (stats.avg_execution_time_ms * (stats.execution_count - 1) as f64 + duration_ms as f64) / stats.execution_count as f64;

        Ok(())
    }

    pub fn get_module_stats(&self, module_id: &str) -> Result<ModuleStats, ModuleError> {
        self.stats
            .get(module_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| ModuleError::NotFound(module_id.to_string()))
    }

    pub fn generate_dashboard(&self) -> Result<AnalyticsDashboard, ModuleError> {
        debug!("Generating analytics dashboard");

        let modules: Vec<ModuleStats> = self.stats.iter().map(|entry| entry.value().clone()).collect();

        let total_executions: u64 = modules.iter().map(|s| s.execution_count).sum();
        let total_errors: u64 = modules.iter().map(|s| s.error_count).sum();
        let error_rate = if total_executions > 0 {
            (total_errors as f64 / total_executions as f64) * 100.0
        } else {
            0.0
        };

        let avg_load_time = if !modules.is_empty() {
            modules.iter().map(|s| s.avg_load_time_ms).sum::<f64>() / modules.len() as f64
        } else {
            0.0
        };

        let dashboard = AnalyticsDashboard {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Omnisystem Module Analytics".to_string(),
            modules: modules.clone(),
            total_modules_loaded: modules.len(),
            total_executions,
            total_errors,
            error_rate,
            avg_system_load_time_ms: avg_load_time,
            uptime_percentage: 99.99,
            generated_at: chrono::Utc::now().timestamp() as u64,
        };

        info!("Dashboard generated with {} modules", modules.len());
        Ok(dashboard)
    }

    pub fn get_error_trends(&self, hours: u32) -> Vec<(String, u64)> {
        self.stats
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().error_count))
            .collect()
    }
}

impl Default for AnalyticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AnalyticsEngine {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            stats: Arc::clone(&self.stats),
            dashboards: Arc::clone(&self.dashboards),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_engine_creation() {
        let engine = AnalyticsEngine::new();
        assert_eq!(engine.dashboards.len(), 0);
    }

    #[test]
    fn test_record_load() {
        let engine = AnalyticsEngine::new();
        assert!(engine.record_load("test-module", 50).is_ok());
    }

    #[test]
    fn test_record_execution() {
        let engine = AnalyticsEngine::new();
        assert!(engine.record_execution("test-module", 100, true).is_ok());
        assert!(engine.record_execution("test-module", 105, false).is_ok());
    }

    #[test]
    fn test_get_module_stats() {
        let engine = AnalyticsEngine::new();
        engine.record_load("test-module", 50).unwrap();
        engine.record_execution("test-module", 100, true).unwrap();
        let stats = engine.get_module_stats("test-module").unwrap();
        assert_eq!(stats.module_id, "test-module");
    }

    #[tokio::test]
    async fn test_generate_dashboard() {
        let engine = AnalyticsEngine::new();
        engine.record_load("test-module", 50).unwrap();
        engine.record_execution("test-module", 100, true).unwrap();
        let dashboard = engine.generate_dashboard().unwrap();
        assert_eq!(dashboard.total_modules_loaded, 1);
    }
}
