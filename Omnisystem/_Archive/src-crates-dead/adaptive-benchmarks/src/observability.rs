use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Comprehensive metrics collection for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPoint {
    pub timestamp: u64,
    pub metric_name: String,
    pub value: f32,
    pub tags: HashMap<String, String>,
    pub scale: u32,
}

pub struct MetricsCollector {
    pub metrics: Arc<Mutex<Vec<MetricsPoint>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Record a metric point
    pub fn record(&self, name: &str, value: f32, scale: u32, tags: HashMap<String, String>) {
        let point = MetricsPoint {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metric_name: name.to_string(),
            value,
            tags,
            scale,
        };

        let mut metrics = self.metrics.lock().unwrap();
        metrics.push(point);
    }

    /// Record latency metric
    pub fn record_latency(&self, latency_ms: f32, scale: u32, batch_size: usize) {
        let mut tags = HashMap::new();
        tags.insert("batch_size".to_string(), batch_size.to_string());

        self.record("latency_ms", latency_ms, scale, tags);
    }

    /// Record memory usage
    pub fn record_memory(&self, memory_mb: f32, scale: u32) {
        self.record("memory_mb", memory_mb, scale, HashMap::new());
    }

    /// Record perplexity
    pub fn record_perplexity(&self, perplexity: f32, scale: u32) {
        self.record("perplexity", perplexity, scale, HashMap::new());
    }

    /// Record throughput
    pub fn record_throughput(&self, tokens_per_sec: f32, scale: u32) {
        self.record("throughput_tokens_sec", tokens_per_sec, scale, HashMap::new());
    }

    /// Get metrics for a specific scale
    pub fn get_scale_metrics(&self, scale: u32) -> Vec<MetricsPoint> {
        let metrics = self.metrics.lock().unwrap();
        metrics.iter()
            .filter(|m| m.scale == scale)
            .cloned()
            .collect()
    }

    /// Aggregate metrics over time
    pub fn aggregate_metrics(&self) -> HashMap<String, AggregatedMetric> {
        let metrics = self.metrics.lock().unwrap();
        let mut aggregated: HashMap<String, Vec<f32>> = HashMap::new();

        for point in metrics.iter() {
            aggregated
                .entry(point.metric_name.clone())
                .or_insert_with(Vec::new)
                .push(point.value);
        }

        let mut result = HashMap::new();
        for (name, values) in aggregated {
            result.insert(
                name,
                AggregatedMetric::from_values(&values),
            );
        }

        result
    }

    /// Export metrics to JSON
    pub fn export_json(&self) -> String {
        let metrics = self.metrics.lock().unwrap();
        serde_json::to_string_pretty(&*metrics).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetric {
    pub count: usize,
    pub mean: f32,
    pub std_dev: f32,
    pub min: f32,
    pub max: f32,
}

impl AggregatedMetric {
    pub fn from_values(values: &[f32]) -> Self {
        if values.is_empty() {
            return Self {
                count: 0,
                mean: 0.0,
                std_dev: 0.0,
                min: 0.0,
                max: 0.0,
            };
        }

        let count = values.len();
        let mean = values.iter().sum::<f32>() / count as f32;

        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f32>() / count as f32;
        let std_dev = variance.sqrt();

        let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        Self {
            count,
            mean,
            std_dev,
            min,
            max,
        }
    }
}

/// Structured logging for benchmarking
pub struct BenchmarkLogger {
    pub logs: Arc<Mutex<Vec<LogEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl BenchmarkLogger {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn info(&self, message: &str, context: HashMap<String, String>) {
        self.log(LogLevel::Info, message, context);
    }

    pub fn warn(&self, message: &str, context: HashMap<String, String>) {
        self.log(LogLevel::Warn, message, context);
    }

    pub fn error(&self, message: &str, context: HashMap<String, String>) {
        self.log(LogLevel::Error, message, context);
    }

    pub fn debug(&self, message: &str, context: HashMap<String, String>) {
        self.log(LogLevel::Debug, message, context);
    }

    fn log(&self, level: LogLevel, message: &str, context: HashMap<String, String>) {
        let entry = LogEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            level,
            message: message.to_string(),
            context,
        };

        let mut logs = self.logs.lock().unwrap();
        logs.push(entry);

        // Also print to console for immediate feedback
        match level {
            LogLevel::Debug => tracing::debug!("{}: {}", message, serde_json::to_string(&context).unwrap_or_default()),
            LogLevel::Info => tracing::info!("{}: {}", message, serde_json::to_string(&context).unwrap_or_default()),
            LogLevel::Warn => tracing::warn!("{}: {}", message, serde_json::to_string(&context).unwrap_or_default()),
            LogLevel::Error => tracing::error!("{}: {}", message, serde_json::to_string(&context).unwrap_or_default()),
        }
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        let logs = self.logs.lock().unwrap();
        logs.clone()
    }

    pub fn get_logs_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        let logs = self.logs.lock().unwrap();
        logs.iter()
            .filter(|l| l.level == level)
            .cloned()
            .collect()
    }

    pub fn export_logs(&self) -> String {
        let logs = self.logs.lock().unwrap();
        serde_json::to_string_pretty(&*logs).unwrap_or_default()
    }
}

/// Dashboard data aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub timestamp: String,
    pub current_scale_distribution: HashMap<u32, usize>,  // scale -> count of active inferences
    pub performance_metrics: HashMap<String, f32>,
    pub cost_analysis: CostAnalysis,
    pub error_rates: HashMap<String, f32>,
    pub recent_logs: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalysis {
    pub total_work_tokens: u64,
    pub cost_per_inference: HashMap<u32, f32>,
    pub cost_breakdown: HashMap<String, f32>,
    pub estimated_monthly_cost: f32,
}

pub struct DashboardBuilder {
    metrics_collector: Arc<MetricsCollector>,
    logger: Arc<BenchmarkLogger>,
}

impl DashboardBuilder {
    pub fn new(metrics: Arc<MetricsCollector>, logger: Arc<BenchmarkLogger>) -> Self {
        Self {
            metrics_collector: metrics,
            logger,
        }
    }

    pub fn build(&self) -> DashboardData {
        let aggregated = self.metrics_collector.aggregate_metrics();

        let mut performance_metrics = HashMap::new();
        for (name, metric) in aggregated.iter() {
            performance_metrics.insert(format!("{}_mean", name), metric.mean);
            performance_metrics.insert(format!("{}_std", name), metric.std_dev);
        }

        let recent_logs = {
            let logs = self.logger.logs.lock().unwrap();
            logs.iter().rev().take(100).cloned().collect()
        };

        DashboardData {
            timestamp: chrono::Local::now().to_rfc3339(),
            current_scale_distribution: HashMap::new(),
            performance_metrics,
            cost_analysis: CostAnalysis {
                total_work_tokens: 0,
                cost_per_inference: HashMap::new(),
                cost_breakdown: HashMap::new(),
                estimated_monthly_cost: 0.0,
            },
            error_rates: HashMap::new(),
            recent_logs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        collector.record_latency(15.5, 100_000_000, 1);
        collector.record_memory(2048.0, 100_000_000);

        let metrics = collector.get_scale_metrics(100_000_000);
        assert_eq!(metrics.len(), 2);
    }

    #[test]
    fn test_aggregated_metric() {
        let values = vec![10.0, 12.0, 14.0, 16.0, 18.0];
        let metric = AggregatedMetric::from_values(&values);

        assert_eq!(metric.count, 5);
        assert_eq!(metric.mean, 14.0);
        assert_eq!(metric.min, 10.0);
        assert_eq!(metric.max, 18.0);
    }

    #[test]
    fn test_benchmark_logger() {
        let logger = BenchmarkLogger::new();

        let mut context = HashMap::new();
        context.insert("scale".to_string(), "100m".to_string());

        logger.info("Starting benchmark", context);

        let logs = logger.get_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].level, LogLevel::Info);
    }

    #[test]
    fn test_logger_by_level() {
        let logger = BenchmarkLogger::new();

        logger.info("info message", HashMap::new());
        logger.warn("warn message", HashMap::new());
        logger.error("error message", HashMap::new());

        let errors = logger.get_logs_by_level(LogLevel::Error);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_dashboard_builder() {
        let metrics = Arc::new(MetricsCollector::new());
        let logger = Arc::new(BenchmarkLogger::new());

        metrics.record_latency(15.5, 100_000_000, 1);

        let builder = DashboardBuilder::new(metrics, logger);
        let dashboard = builder.build();

        assert!(!dashboard.performance_metrics.is_empty());
    }
}

use chrono;
use tracing;
