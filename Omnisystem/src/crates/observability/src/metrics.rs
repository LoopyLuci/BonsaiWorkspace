use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Prometheus metrics collector
pub struct MetricsCollector {
    counters: Arc<RwLock<HashMap<String, u64>>>,
    gauges: Arc<RwLock<HashMap<String, f64>>>,
    histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

/// Recorded metric point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record an operation (counter + histogram)
    pub fn record(&self, operation: &str, latency_ms: f64, success: bool) {
        // Increment counter
        let mut counters = self.counters.write();
        let counter_name = format!("operations_total{{operation=\"{}\"}}", operation);
        *counters.entry(counter_name).or_insert(0) += 1;

        // Record latency in histogram
        let mut histograms = self.histograms.write();
        histograms
            .entry(format!("operation_duration_ms{{operation=\"{}\"}}", operation))
            .or_insert_with(Vec::new)
            .push(latency_ms);

        // Track errors if failed
        if !success {
            let error_counter = format!("errors_total{{operation=\"{}\"}}", operation);
            *counters.entry(error_counter).or_insert(0) += 1;
        }
    }

    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write();
        gauges.insert(name.to_string(), value);
    }

    /// Increment a counter
    pub fn increment_counter(&self, name: &str, amount: u64) {
        let mut counters = self.counters.write();
        *counters.entry(name.to_string()).or_insert(0) += amount;
    }

    /// Get counter value
    pub fn get_counter(&self, name: &str) -> Option<u64> {
        self.counters.read().get(name).copied()
    }

    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> Option<f64> {
        self.gauges.read().get(name).copied()
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> Result<String, String> {
        let mut output = String::new();

        // Export counters
        output.push_str("# HELP operations_total Total number of operations\n");
        output.push_str("# TYPE operations_total counter\n");
        for (name, value) in self.counters.read().iter() {
            output.push_str(&format!("{} {}\n", name, value));
        }

        // Export gauges
        output.push_str("# HELP gauge_values Current gauge values\n");
        output.push_str("# TYPE gauge_values gauge\n");
        for (name, value) in self.gauges.read().iter() {
            output.push_str(&format!("{} {}\n", name, value));
        }

        // Export histograms (as summaries for simplicity)
        output.push_str("# HELP operation_duration_ms Operation duration in milliseconds\n");
        output.push_str("# TYPE operation_duration_ms histogram\n");
        for (name, values) in self.histograms.read().iter() {
            if !values.is_empty() {
                let sum: f64 = values.iter().sum();
                output.push_str(&format!("{}{{le=\"+Inf\"}} {}\n", name, values.len()));
                output.push_str(&format!("{}_sum {}\n", name, sum));
                output.push_str(&format!("{}_count {}\n", name, values.len()));
            }
        }

        Ok(output)
    }

    /// Get all metrics as JSON
    pub fn export_json(&self) -> Result<String, String> {
        let counters = self.counters.read().clone();
        let gauges = self.gauges.read().clone();
        let histograms = self.histograms.read().clone();

        let metrics = serde_json::json!({
            "counters": counters,
            "gauges": gauges,
            "histograms": histograms,
        });

        serde_json::to_string_pretty(&metrics)
            .map_err(|e| format!("JSON serialization failed: {}", e))
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_operation() {
        let collector = MetricsCollector::new();
        collector.record("test_op", 10.5, true);
        assert_eq!(collector.get_counter("operations_total{operation=\"test_op\"}"), Some(1));
    }

    #[test]
    fn test_gauge() {
        let collector = MetricsCollector::new();
        collector.set_gauge("test_gauge", 42.0);
        assert_eq!(collector.get_gauge("test_gauge"), Some(42.0));
    }
}
