//! Performance Optimization and Profiling
//!
//! This module provides profiling, caching optimization, and parallel execution tuning
//! to achieve <50ms end-to-end latency targets.

use crate::error::{VerificationError, VerificationResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use std::sync::RwLock;
use std::collections::HashMap;

/// Profiling result for a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingResult {
    /// Unique identifier
    pub id: Uuid,
    /// Component name
    pub component: String,
    /// Timestamp of profiling
    pub timestamp: DateTime<Utc>,
    /// Total execution time in milliseconds
    pub total_ms: f64,
    /// Minimum execution time in milliseconds
    pub min_ms: f64,
    /// Maximum execution time in milliseconds
    pub max_ms: f64,
    /// Average execution time in milliseconds
    pub avg_ms: f64,
    /// Median execution time in milliseconds
    pub median_ms: f64,
    /// 95th percentile latency in milliseconds
    pub p95_ms: f64,
    /// 99th percentile latency in milliseconds
    pub p99_ms: f64,
    /// Number of samples
    pub samples: usize,
    /// Detected bottleneck?
    pub is_bottleneck: bool,
    /// Bottleneck description
    pub bottleneck_reason: Option<String>,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Total cache operations
    pub total_ops: u64,
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn calculate_hit_rate(&mut self) {
        self.total_ops = self.hits + self.misses;
        self.hit_rate = if self.total_ops == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_ops as f64
        };
    }
}

/// Performance optimizer for AHF components
pub struct PerformanceOptimizer {
    /// Profiling results
    results: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    /// Cache statistics
    cache_stats: Arc<RwLock<CacheStats>>,
    /// Proof cache
    proof_cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Schema cache
    schema_cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Latency target in milliseconds
    latency_target_ms: f64,
}

impl PerformanceOptimizer {
    /// Create a new performance optimizer
    pub fn new(latency_target_ms: f64) -> Self {
        PerformanceOptimizer {
            results: Arc::new(RwLock::new(HashMap::new())),
            cache_stats: Arc::new(RwLock::new(CacheStats {
                hits: 0,
                misses: 0,
                total_ops: 0,
                hit_rate: 0.0,
            })),
            proof_cache: Arc::new(RwLock::new(HashMap::new())),
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
            latency_target_ms,
        }
    }

    /// Record a profiling measurement
    pub fn record_measurement(&self, component: &str, time_ms: f64) {
        let mut results = self.results.write().unwrap();
        results.entry(component.to_string())
            .or_insert_with(Vec::new)
            .push(time_ms);
    }

    /// Get profiling results for a component
    pub fn get_profiling_result(&self, component: &str) -> VerificationResult<ProfilingResult> {
        let results = self.results.read().unwrap();
        let times = results
            .get(component)
            .cloned()
            .ok_or_else(|| {
                VerificationError::ProfilingError(format!("No profiling data for {}", component))
            })?;

        if times.is_empty() {
            return Err(VerificationError::ProfilingError(
                "No measurements recorded".to_string(),
            ));
        }

        let min_ms = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_ms = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg_ms = times.iter().sum::<f64>() / times.len() as f64;
        let total_ms = times.iter().sum::<f64>();

        // Calculate percentiles
        let mut sorted = times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_ms = sorted[sorted.len() / 2];
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        let p95_ms = sorted.get(p95_idx).cloned().unwrap_or(max_ms);
        let p99_ms = sorted.get(p99_idx).cloned().unwrap_or(max_ms);

        // Detect bottleneck
        let is_bottleneck = avg_ms > self.latency_target_ms * 0.5; // Bottleneck if using >50% of budget
        let bottleneck_reason = if is_bottleneck {
            Some(format!(
                "Average latency {:.2}ms exceeds budget {:.2}ms",
                avg_ms, self.latency_target_ms
            ))
        } else {
            None
        };

        Ok(ProfilingResult {
            id: Uuid::new_v4(),
            component: component.to_string(),
            timestamp: Utc::now(),
            total_ms,
            min_ms,
            max_ms,
            avg_ms,
            median_ms,
            p95_ms,
            p99_ms,
            samples: times.len(),
            is_bottleneck,
            bottleneck_reason,
        })
    }

    /// Cache a proof
    pub fn cache_proof(&self, key: String, value: Vec<u8>) {
        self.proof_cache.write().unwrap().insert(key, value);
        let mut stats = self.cache_stats.write().unwrap();
        stats.hits += 1;
        stats.calculate_hit_rate();
    }

    /// Retrieve cached proof
    pub fn get_cached_proof(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.proof_cache.read().unwrap().get(key).cloned() {
            let mut stats = self.cache_stats.write().unwrap();
            stats.hits += 1;
            stats.calculate_hit_rate();
            Some(entry)
        } else {
            let mut stats = self.cache_stats.write().unwrap();
            stats.misses += 1;
            stats.calculate_hit_rate();
            None
        }
    }

    /// Cache a schema
    pub fn cache_schema(&self, key: String, value: Vec<u8>) {
        self.schema_cache.write().unwrap().insert(key, value);
    }

    /// Retrieve cached schema
    pub fn get_cached_schema(&self, key: &str) -> Option<Vec<u8>> {
        self.schema_cache.read().unwrap().get(key).cloned()
    }

    /// Get cache statistics
    pub fn cache_statistics(&self) -> CacheStats {
        self.cache_stats.read().unwrap().clone()
    }

    /// Clear all caches
    pub fn clear_caches(&self) {
        self.proof_cache.write().unwrap().clear();
        self.schema_cache.write().unwrap().clear();
    }

    /// Get all profiling results
    pub fn all_profiling_results(&self) -> VerificationResult<Vec<ProfilingResult>> {
        let mut results = Vec::new();

        let results_map = self.results.read().unwrap();
        for key in results_map.keys() {
            if let Ok(result) = self.get_profiling_result(key) {
                results.push(result);
            }
        }

        if results.is_empty() {
            return Err(VerificationError::ProfilingError(
                "No profiling results available".to_string(),
            ));
        }

        Ok(results)
    }

    /// Check if system meets latency target
    pub fn meets_latency_target(&self) -> VerificationResult<bool> {
        let results = self.all_profiling_results()?;
        Ok(results.iter().all(|r| r.avg_ms <= self.latency_target_ms))
    }

    /// Get performance summary
    pub fn performance_summary(&self) -> VerificationResult<PerformanceSummary> {
        let results = self.all_profiling_results()?;
        let bottlenecks: Vec<_> = results.iter().filter(|r| r.is_bottleneck).cloned().collect();
        let avg_latency = results.iter().map(|r| r.avg_ms).sum::<f64>() / results.len() as f64;
        let cache_stats = self.cache_statistics();

        Ok(PerformanceSummary {
            average_latency_ms: avg_latency,
            target_latency_ms: self.latency_target_ms,
            meets_target: avg_latency <= self.latency_target_ms,
            bottleneck_count: bottlenecks.len(),
            bottlenecks,
            cache_stats,
            total_components_profiled: results.len(),
        })
    }
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub average_latency_ms: f64,
    pub target_latency_ms: f64,
    pub meets_target: bool,
    pub bottleneck_count: usize,
    pub bottlenecks: Vec<ProfilingResult>,
    pub cache_stats: CacheStats,
    pub total_components_profiled: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_optimizer_creation() {
        let optimizer = PerformanceOptimizer::new(50.0);
        assert_eq!(optimizer.latency_target_ms, 50.0);
    }

    #[test]
    fn test_record_measurements() {
        let optimizer = PerformanceOptimizer::new(50.0);

        optimizer.record_measurement("verifier", 10.5);
        optimizer.record_measurement("verifier", 12.3);
        optimizer.record_measurement("verifier", 11.8);

        let result = optimizer.get_profiling_result("verifier").unwrap();
        assert_eq!(result.samples, 3);
        assert!(result.avg_ms > 10.0 && result.avg_ms < 13.0);
        assert_eq!(result.min_ms, 10.5);
        assert_eq!(result.max_ms, 12.3);
    }

    #[test]
    fn test_cache_operations() {
        let optimizer = PerformanceOptimizer::new(50.0);

        optimizer.cache_proof("test_key".to_string(), vec![1, 2, 3]);
        let cached = optimizer.get_cached_proof("test_key");
        assert!(cached.is_some());

        let stats = optimizer.cache_statistics();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_bottleneck_detection() {
        let optimizer = PerformanceOptimizer::new(20.0);

        for _ in 0..5 {
            optimizer.record_measurement("slow_component", 15.0);
        }

        let result = optimizer.get_profiling_result("slow_component").unwrap();
        assert!(!result.is_bottleneck); // 15ms < 10ms (50% of 20ms)

        for _ in 0..5 {
            optimizer.record_measurement("very_slow", 12.0);
        }

        let result2 = optimizer.get_profiling_result("very_slow").unwrap();
        assert!(result2.is_bottleneck); // 12ms > 10ms
    }

    #[test]
    fn test_percentile_calculations() {
        let optimizer = PerformanceOptimizer::new(100.0);

        for i in 1..=100 {
            optimizer.record_measurement("test", i as f64);
        }

        let result = optimizer.get_profiling_result("test").unwrap();
        assert!(result.p95_ms >= 95.0);
        assert!(result.p99_ms >= 99.0);
    }

    #[test]
    fn test_cache_stats() {
        let optimizer = PerformanceOptimizer::new(50.0);

        optimizer.cache_proof("key1".to_string(), vec![1, 2, 3]);
        optimizer.get_cached_proof("key1");
        optimizer.get_cached_proof("key1");
        optimizer.get_cached_proof("missing");

        let stats = optimizer.cache_statistics();
        assert!(stats.hit_rate > 0.5);
    }
}
