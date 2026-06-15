//! Metrics Collection and Analysis
//!
//! Collects latency histograms, throughput measurements, CPU/memory profiling,
//! and cache behavior metrics during kernel stress testing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Latency histogram bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub count: u64,
}

impl HistogramBucket {
    /// Create a new histogram bucket
    pub fn new(lower: f64, upper: f64) -> Self {
        Self {
            lower_bound: lower,
            upper_bound: upper,
            count: 0,
        }
    }
}

/// Latency histogram with percentiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyHistogram {
    pub name: String,
    pub buckets: Vec<HistogramBucket>,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
    pub p999: f64,
    pub total_count: u64,
}

impl LatencyHistogram {
    /// Create a new latency histogram
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();

        // Create logarithmic buckets: 0.1µs to 100ms
        let mut buckets = Vec::new();
        let mut lower = 0.1;

        while lower < 100_000.0 {
            let upper = lower * 10.0;
            buckets.push(HistogramBucket::new(lower, upper));
            lower = upper;
        }

        Self {
            name,
            buckets,
            min: f64::INFINITY,
            max: 0.0,
            mean: 0.0,
            p50: 0.0,
            p90: 0.0,
            p99: 0.0,
            p999: 0.0,
            total_count: 0,
        }
    }

    /// Record a latency value
    pub fn record(&mut self, value: f64) {
        self.total_count += 1;

        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }

        // Update mean incrementally
        self.mean = (self.mean * (self.total_count as f64 - 1.0) + value) / self.total_count as f64;

        // Find and increment appropriate bucket
        for bucket in &mut self.buckets {
            if value >= bucket.lower_bound && value < bucket.upper_bound {
                bucket.count += 1;
                break;
            }
        }
    }

    /// Calculate percentiles
    pub fn calculate_percentiles(&mut self) {
        if self.total_count == 0 {
            return;
        }

        let mut cumulative = 0u64;

        for percentile_target in &[50.0, 90.0, 99.0, 99.9] {
            let target_count = (self.total_count as f64 * (percentile_target / 100.0)) as u64;

            cumulative = 0;
            for bucket in &self.buckets {
                cumulative += bucket.count;

                if cumulative >= target_count {
                    let percentile_value = (bucket.lower_bound + bucket.upper_bound) / 2.0;

                    match percentile_target {
                        50.0 => self.p50 = percentile_value,
                        90.0 => self.p90 = percentile_value,
                        99.0 => self.p99 = percentile_value,
                        99.9 => self.p999 = percentile_value,
                        _ => {}
                    }

                    break;
                }
            }
        }
    }
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub throughput_ops_per_sec: f64,
    pub avg_latency_us: f64,
    pub p99_latency_us: f64,
    pub p999_latency_us: f64,
    pub cpu_utilization_percent: f64,
    pub memory_usage_mb: u64,
    pub context_switches_per_sec: f64,
    pub cache_hit_ratio: f64,
}

/// Resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub cpu_usage: f64,
    pub memory_allocated: u64,
    pub memory_peak: u64,
    pub disk_io_bytes: u64,
    pub network_io_bytes: u64,
    pub threads_created: u64,
}

/// Metrics collector
#[derive(Debug)]
pub struct MetricsCollector {
    histograms: BTreeMap<String, LatencyHistogram>,
    throughput_counts: BTreeMap<String, u64>,
    cpu_usage: f64,
    memory_allocated: u64,
    memory_peak: u64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            histograms: BTreeMap::new(),
            throughput_counts: BTreeMap::new(),
            cpu_usage: 0.0,
            memory_allocated: 0,
            memory_peak: 0,
        }
    }

    /// Record a latency value
    pub fn record_latency(&mut self, name: impl Into<String>, value_us: f64) {
        let name = name.into();

        self.histograms
            .entry(name)
            .or_insert_with(|| LatencyHistogram::new(""))
            .record(value_us);
    }

    /// Record a throughput count
    pub fn record_throughput(&mut self, name: impl Into<String>) {
        let name = name.into();
        *self.throughput_counts.entry(name).or_insert(0) += 1;
    }

    /// Update CPU usage
    pub fn set_cpu_usage(&mut self, percent: f64) {
        self.cpu_usage = percent;
    }

    /// Update memory usage
    pub fn set_memory_usage(&mut self, allocated: u64, peak: u64) {
        self.memory_allocated = allocated;
        self.memory_peak = peak;
    }

    /// Get latency histogram
    pub fn get_histogram(&mut self, name: &str) -> Option<&mut LatencyHistogram> {
        self.histograms.get_mut(name)
    }

    /// Get all histograms
    pub fn get_histograms(&mut self) -> Vec<LatencyHistogram> {
        for histogram in self.histograms.values_mut() {
            histogram.calculate_percentiles();
        }
        self.histograms.values().cloned().collect()
    }

    /// Get throughput count
    pub fn get_throughput_count(&self, name: &str) -> u64 {
        self.throughput_counts.get(name).copied().unwrap_or(0)
    }

    /// Get resource metrics
    pub fn get_resource_metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            cpu_usage: self.cpu_usage,
            memory_allocated: self.memory_allocated,
            memory_peak: self.memory_peak,
            disk_io_bytes: 0,
            network_io_bytes: 0,
            threads_created: 0,
        }
    }

    /// Generate summary
    pub fn generate_summary(&mut self) -> String {
        let mut summary = String::new();
        summary.push_str("=== Metrics Summary ===\n");

        for histogram in self.get_histograms() {
            summary.push_str(&format!(
                "{}: min={:.2}µs, mean={:.2}µs, p50={:.2}µs, p99={:.2}µs, max={:.2}µs, samples={}\n",
                histogram.name, histogram.min, histogram.mean, histogram.p50, histogram.p99, histogram.max, histogram.total_count
            ));
        }

        let resources = self.get_resource_metrics();
        summary.push_str(&format!(
            "CPU: {:.1}%, Memory: {}/{} MB\n",
            resources.cpu_usage, resources.memory_allocated / 1024 / 1024, resources.memory_peak / 1024 / 1024
        ));

        summary
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
    fn test_histogram_creation() {
        let hist = LatencyHistogram::new("test");
        assert_eq!(hist.name, "test");
        assert!(!hist.buckets.is_empty());
    }

    #[test]
    fn test_histogram_record() {
        let mut hist = LatencyHistogram::new("test");
        hist.record(50.0);
        hist.record(100.0);
        hist.record(150.0);

        assert_eq!(hist.total_count, 3);
        assert!(hist.min <= 50.0);
        assert!(hist.max >= 150.0);
    }

    #[test]
    fn test_histogram_percentiles() {
        let mut hist = LatencyHistogram::new("test");

        for i in 1..=100 {
            hist.record(i as f64);
        }

        hist.calculate_percentiles();

        assert!(hist.p50 > 0.0);
        assert!(hist.p99 > 0.0);
    }

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.histograms.len(), 0);
        assert_eq!(collector.throughput_counts.len(), 0);
    }

    #[test]
    fn test_metrics_collector_latency() {
        let mut collector = MetricsCollector::new();
        collector.record_latency("test", 50.0);
        collector.record_latency("test", 100.0);

        let histogram = collector.get_histogram("test");
        assert!(histogram.is_some());
        assert_eq!(histogram.unwrap().total_count, 2);
    }

    #[test]
    fn test_metrics_collector_throughput() {
        let mut collector = MetricsCollector::new();
        collector.record_throughput("ops");
        collector.record_throughput("ops");
        collector.record_throughput("ops");

        let count = collector.get_throughput_count("ops");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_metrics_collector_summary() {
        let mut collector = MetricsCollector::new();
        collector.record_latency("test", 50.0);
        collector.record_latency("test", 100.0);
        collector.set_cpu_usage(50.0);
        collector.set_memory_usage(100 * 1024 * 1024, 200 * 1024 * 1024);

        let summary = collector.generate_summary();
        assert!(summary.contains("Metrics Summary"));
        assert!(summary.contains("CPU:"));
    }
}
