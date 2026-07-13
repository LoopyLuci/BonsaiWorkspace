//! Performance metrics tracking with statistical analysis

use crate::errors::{CIError, CIResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Single performance metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub latencies: Vec<f64>,           // ms, from 10 runs
    pub throughputs: Vec<f64>,         // ops/sec
    pub memory_peak: f64,              // MB
    pub cpu_average: f64,              // percentage
    pub io_operations: u64,
    pub timestamp: DateTime<Utc>,
}

/// Aggregated metrics with statistical analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsBucket {
    pub name: String,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub stddev: f64,
    pub count: usize,
}

/// Complete performance metrics record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub latency_bucket: MetricsBucket,
    pub throughput_bucket: MetricsBucket,
    pub memory_peak: f64,
    pub cpu_average: f64,
    pub io_operations: u64,
    pub timestamp: DateTime<Utc>,
}

impl MetricsSnapshot {
    pub fn new() -> Self {
        Self {
            latencies: Vec::new(),
            throughputs: Vec::new(),
            memory_peak: 0.0,
            cpu_average: 0.0,
            io_operations: 0,
            timestamp: Utc::now(),
        }
    }

    /// Add latency sample (in ms)
    pub fn add_latency(&mut self, latency_ms: f64) {
        self.latencies.push(latency_ms);
    }

    /// Add throughput sample (ops/sec)
    pub fn add_throughput(&mut self, throughput: f64) {
        self.throughputs.push(throughput);
    }

    /// Set peak memory usage (MB)
    pub fn set_memory_peak(&mut self, mb: f64) {
        self.memory_peak = mb;
    }

    /// Set average CPU usage (0-100%)
    pub fn set_cpu_average(&mut self, percent: f64) {
        self.cpu_average = percent.max(0.0).min(100.0);
    }

    /// Set IO operations count
    pub fn set_io_operations(&mut self, count: u64) {
        self.io_operations = count;
    }

    /// Check if snapshot is complete (has all required metrics)
    pub fn is_complete(&self) -> bool {
        !self.latencies.is_empty()
            && !self.throughputs.is_empty()
            && self.memory_peak > 0.0
            && self.io_operations > 0
    }
}

impl Default for MetricsSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsBucket {
    /// Create bucket from raw data points
    pub fn from_samples(name: &str, samples: &[f64]) -> CIResult<Self> {
        if samples.is_empty() {
            return Err(CIError::InvalidMetric(format!(
                "Cannot create bucket from empty samples: {}",
                name
            )));
        }

        let sorted = {
            let mut s = samples.to_vec();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            s
        };

        let min = *sorted.first().unwrap_or(&0.0);
        let max = *sorted.last().unwrap_or(&0.0);
        let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;

        let variance = sorted
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / sorted.len() as f64;
        let stddev = variance.sqrt();

        let p50 = percentile(&sorted, 50.0)?;
        let p95 = percentile(&sorted, 95.0)?;
        let p99 = percentile(&sorted, 99.0)?;

        Ok(Self {
            name: name.to_string(),
            p50,
            p95,
            p99,
            min,
            max,
            mean,
            stddev,
            count: samples.len(),
        })
    }

    /// Get all statistics as a map
    pub fn as_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert(format!("{}_p50", self.name), self.p50);
        map.insert(format!("{}_p95", self.name), self.p95);
        map.insert(format!("{}_p99", self.name), self.p99);
        map.insert(format!("{}_min", self.name), self.min);
        map.insert(format!("{}_max", self.name), self.max);
        map.insert(format!("{}_mean", self.name), self.mean);
        map.insert(format!("{}_stddev", self.name), self.stddev);
        map
    }
}

impl PerformanceMetrics {
    /// Create metrics from snapshot
    pub fn from_snapshot(snapshot: &MetricsSnapshot) -> CIResult<Self> {
        if !snapshot.is_complete() {
            return Err(CIError::MetricsCollectionFailed(
                "Snapshot is not complete".to_string(),
            ));
        }

        let latency_bucket = MetricsBucket::from_samples("latency", &snapshot.latencies)?;
        let throughput_bucket = MetricsBucket::from_samples("throughput", &snapshot.throughputs)?;

        info!(
            "Metrics created: latency p99={:.2}ms, throughput mean={:.0} ops/sec",
            latency_bucket.p99, throughput_bucket.mean
        );

        Ok(Self {
            latency_bucket,
            throughput_bucket,
            memory_peak: snapshot.memory_peak,
            cpu_average: snapshot.cpu_average,
            io_operations: snapshot.io_operations,
            timestamp: Utc::now(),
        })
    }

    /// Get all metrics as flat map for comparison
    pub fn as_flat_map(&self) -> HashMap<String, f64> {
        let mut map = self.latency_bucket.as_map();
        map.extend(self.throughput_bucket.as_map());
        map.insert("memory_peak".to_string(), self.memory_peak);
        map.insert("cpu_average".to_string(), self.cpu_average);
        map.insert("io_operations".to_string(), self.io_operations as f64);
        map
    }

    /// Calculate difference from baseline (percentage)
    pub fn percentage_diff(&self, baseline: &Self, metric: &str) -> CIResult<f64> {
        let current = self.get_metric(metric)?;
        let baseline_value = baseline.get_metric(metric)?;

        if baseline_value == 0.0 {
            return Ok(0.0);
        }

        Ok(((current - baseline_value) / baseline_value) * 100.0)
    }

    /// Get single metric by name
    pub fn get_metric(&self, name: &str) -> CIResult<f64> {
        match name {
            "latency_p50" => Ok(self.latency_bucket.p50),
            "latency_p95" => Ok(self.latency_bucket.p95),
            "latency_p99" => Ok(self.latency_bucket.p99),
            "latency_mean" => Ok(self.latency_bucket.mean),
            "throughput_mean" => Ok(self.throughput_bucket.mean),
            "memory_peak" => Ok(self.memory_peak),
            "cpu_average" => Ok(self.cpu_average),
            "io_operations" => Ok(self.io_operations as f64),
            _ => Err(CIError::InvalidMetric(name.to_string())),
        }
    }
}

/// Calculate percentile from sorted data
fn percentile(sorted: &[f64], p: f64) -> CIResult<f64> {
    if sorted.is_empty() {
        return Err(CIError::InvalidMetric("Cannot calculate percentile of empty data".to_string()));
    }

    if p < 0.0 || p > 100.0 {
        return Err(CIError::InvalidMetric(format!(
            "Invalid percentile: {}",
            p
        )));
    }

    let index = (p / 100.0) * (sorted.len() as f64 - 1.0);
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    let weight = index.fract();

    if lower == upper {
        Ok(sorted[lower])
    } else if upper >= sorted.len() {
        Ok(sorted[sorted.len() - 1])
    } else {
        Ok(sorted[lower] * (1.0 - weight) + sorted[upper] * weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_snapshot_creation() {
        let mut snapshot = MetricsSnapshot::new();
        snapshot.add_latency(10.0);
        snapshot.add_latency(15.0);
        snapshot.add_throughput(100.0);
        snapshot.set_memory_peak(512.0);
        snapshot.set_cpu_average(45.0);
        snapshot.set_io_operations(42);

        assert_eq!(snapshot.latencies.len(), 2);
        assert_eq!(snapshot.throughputs.len(), 1);
        assert_eq!(snapshot.memory_peak, 512.0);
        assert!(snapshot.is_complete());
    }

    #[test]
    fn test_metrics_bucket_from_samples() {
        let samples = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let bucket = MetricsBucket::from_samples("test", &samples).unwrap();

        assert_eq!(bucket.min, 10.0);
        assert_eq!(bucket.max, 50.0);
        assert_eq!(bucket.mean, 30.0);
        assert_eq!(bucket.count, 5);
    }

    #[test]
    fn test_percentile_calculation() {
        let samples = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        let bucket = MetricsBucket::from_samples("test", &samples).unwrap();

        assert_eq!(bucket.p50, 55.0); // median
        assert!(bucket.p95 > 90.0);
        assert!(bucket.p99 > 98.0);
    }

    #[test]
    fn test_metrics_bucket_as_map() {
        let samples = vec![10.0, 20.0, 30.0];
        let bucket = MetricsBucket::from_samples("latency", &samples).unwrap();
        let map = bucket.as_map();

        assert!(map.contains_key("latency_p50"));
        assert!(map.contains_key("latency_p99"));
        assert_eq!(map.len(), 7);
    }

    #[test]
    fn test_performance_metrics_from_snapshot() {
        let mut snapshot = MetricsSnapshot::new();
        for i in 0..10 {
            snapshot.add_latency(100.0 + i as f64);
        }
        for i in 0..5 {
            snapshot.add_throughput(1000.0 + i as f64 * 100.0);
        }
        snapshot.set_memory_peak(512.0);
        snapshot.set_cpu_average(45.0);
        snapshot.set_io_operations(42);

        let metrics = PerformanceMetrics::from_snapshot(&snapshot).unwrap();
        assert!(metrics.latency_bucket.p99 > 0.0);
        assert!(metrics.throughput_bucket.mean > 0.0);
    }

    #[test]
    fn test_metrics_percentage_diff() {
        let mut snapshot1 = MetricsSnapshot::new();
        for _ in 0..5 {
            snapshot1.add_latency(100.0);
            snapshot1.add_throughput(1000.0);
        }
        snapshot1.set_memory_peak(512.0);
        snapshot1.set_cpu_average(45.0);
        snapshot1.set_io_operations(42);

        let mut snapshot2 = MetricsSnapshot::new();
        for _ in 0..5 {
            snapshot2.add_latency(110.0); // 10% increase
            snapshot2.add_throughput(1000.0);
        }
        snapshot2.set_memory_peak(512.0);
        snapshot2.set_cpu_average(45.0);
        snapshot2.set_io_operations(42);

        let m1 = PerformanceMetrics::from_snapshot(&snapshot1).unwrap();
        let m2 = PerformanceMetrics::from_snapshot(&snapshot2).unwrap();

        let diff = m2.percentage_diff(&m1, "latency_p50").unwrap();
        assert!(diff > 9.0 && diff < 11.0); // approximately 10%
    }

    #[test]
    fn test_metrics_get_metric() {
        let mut snapshot = MetricsSnapshot::new();
        for _ in 0..5 {
            snapshot.add_latency(100.0);
            snapshot.add_throughput(1000.0);
        }
        snapshot.set_memory_peak(512.0);
        snapshot.set_cpu_average(45.0);
        snapshot.set_io_operations(42);

        let metrics = PerformanceMetrics::from_snapshot(&snapshot).unwrap();

        let latency_p99 = metrics.get_metric("latency_p99").unwrap();
        assert!(latency_p99 > 0.0);

        let memory = metrics.get_metric("memory_peak").unwrap();
        assert_eq!(memory, 512.0);

        let invalid = metrics.get_metric("invalid_metric");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_metrics_flat_map() {
        let mut snapshot = MetricsSnapshot::new();
        for _ in 0..5 {
            snapshot.add_latency(100.0);
            snapshot.add_throughput(1000.0);
        }
        snapshot.set_memory_peak(512.0);
        snapshot.set_cpu_average(45.0);
        snapshot.set_io_operations(42);

        let metrics = PerformanceMetrics::from_snapshot(&snapshot).unwrap();
        let map = metrics.as_flat_map();

        assert!(map.contains_key("latency_p50"));
        assert!(map.contains_key("throughput_mean"));
        assert!(map.contains_key("memory_peak"));
        assert!(map["memory_peak"] == 512.0);
    }

    #[test]
    fn test_invalid_percentile() {
        let result = percentile(&[1.0, 2.0, 3.0], 150.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_metrics_bucket() {
        let result = MetricsBucket::from_samples("test", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cpu_clamping() {
        let mut snapshot = MetricsSnapshot::new();
        snapshot.set_cpu_average(150.0); // Should clamp to 100
        assert_eq!(snapshot.cpu_average, 100.0);

        snapshot.set_cpu_average(-10.0); // Should clamp to 0
        assert_eq!(snapshot.cpu_average, 0.0);
    }
}
