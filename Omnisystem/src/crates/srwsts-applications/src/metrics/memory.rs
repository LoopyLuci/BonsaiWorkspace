//! Memory profiling and leak detection

use dashmap::DashMap;
use std::sync::Arc;

/// Memory usage record
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub bytes: u64,
    pub label: String,
}

/// Memory profiling collector
#[derive(Debug, Clone)]
pub struct MemoryProfile {
    records: Arc<DashMap<String, Vec<MemoryRecord>>>,
    peak_bytes: Arc<std::sync::atomic::AtomicU64>,
}

impl MemoryProfile {
    /// Create a new memory profile
    pub fn new() -> Self {
        Self {
            records: Arc::new(DashMap::new()),
            peak_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Record memory usage
    pub fn record(&self, label: impl Into<String>, bytes: u64) {
        let label_str = label.into();

        // Update peak
        let current_peak = self.peak_bytes.load(std::sync::atomic::Ordering::Relaxed);
        if bytes > current_peak {
            let _ = self.peak_bytes.compare_exchange(
                current_peak,
                bytes,
                std::sync::atomic::Ordering::Release,
                std::sync::atomic::Ordering::Relaxed,
            );
        }

        // Record measurement
        self.records
            .entry(label_str.clone())
            .or_insert_with(Vec::new)
            .push(MemoryRecord {
                timestamp: chrono::Utc::now(),
                bytes,
                label: label_str,
            });
    }

    /// Get peak memory usage
    pub fn peak_bytes(&self) -> u64 {
        self.peak_bytes.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Get average memory usage
    pub fn average_bytes(&self) -> u64 {
        let records: Vec<_> = self
            .records
            .iter()
            .flat_map(|entry| entry.value().clone())
            .collect();

        if records.is_empty() {
            0
        } else {
            let total: u64 = records.iter().map(|r| r.bytes).sum();
            total / records.len() as u64
        }
    }

    /// Get memory growth rate (bytes per measurement)
    pub fn growth_rate(&self) -> f64 {
        let records: Vec<_> = self
            .records
            .iter()
            .flat_map(|entry| entry.value().clone())
            .collect();

        if records.len() < 2 {
            return 0.0;
        }

        let mut sorted = records.clone();
        sorted.sort_by_key(|r| r.timestamp);

        let first_bytes = sorted.first().map(|r| r.bytes as f64).unwrap_or(0.0);
        let last_bytes = sorted.last().map(|r| r.bytes as f64).unwrap_or(0.0);

        (last_bytes - first_bytes) / sorted.len() as f64
    }

    /// Detect potential memory leaks
    pub fn detect_leaks(&self) -> Vec<MemoryLeak> {
        let mut leaks = Vec::new();

        for entry in self.records.iter() {
            let measurements = entry.value();
            if measurements.len() < 5 {
                continue;
            }

            // Check for monotonic increase
            let growth_rate = self.calculate_growth_rate(measurements);
            if growth_rate > 1024.0 * 1024.0 {
                // More than 1MB per measurement
                leaks.push(MemoryLeak {
                    label: entry.key().clone(),
                    growth_rate_bytes_per_sec: growth_rate,
                    starting_bytes: measurements.first().map(|r| r.bytes).unwrap_or(0),
                    ending_bytes: measurements.last().map(|r| r.bytes).unwrap_or(0),
                });
            }
        }

        leaks
    }

    fn calculate_growth_rate(&self, measurements: &[MemoryRecord]) -> f64 {
        if measurements.len() < 2 {
            return 0.0;
        }

        let first = measurements.first().unwrap();
        let last = measurements.last().unwrap();

        let time_diff = (last.timestamp - first.timestamp).num_seconds().max(1);
        let bytes_diff = (last.bytes as i64 - first.bytes as i64) as f64;

        bytes_diff / time_diff as f64
    }

    /// Generate summary
    pub fn summary(&self) -> super::MemorySummary {
        super::MemorySummary {
            peak_mb: self.peak_bytes() / (1024 * 1024),
            average_mb: self.average_bytes() / (1024 * 1024),
        }
    }
}

impl Default for MemoryProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// Detected memory leak
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryLeak {
    pub label: String,
    pub growth_rate_bytes_per_sec: f64,
    pub starting_bytes: u64,
    pub ending_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_profile_creation() {
        let profile = MemoryProfile::new();
        assert_eq!(profile.peak_bytes(), 0);
    }

    #[test]
    fn test_memory_record() {
        let profile = MemoryProfile::new();
        profile.record("heap", 1024 * 1024 * 512); // 512 MB
        assert_eq!(profile.peak_bytes(), 512 * 1024 * 1024);
    }

    #[test]
    fn test_memory_peak_tracking() {
        let profile = MemoryProfile::new();
        profile.record("heap", 1024);
        profile.record("heap", 2048);
        profile.record("heap", 512);
        assert_eq!(profile.peak_bytes(), 2048);
    }

    #[test]
    fn test_memory_average() {
        let profile = MemoryProfile::new();
        profile.record("heap", 100);
        profile.record("heap", 200);
        profile.record("heap", 300);
        assert_eq!(profile.average_bytes(), 200);
    }

    #[test]
    fn test_memory_summary() {
        let profile = MemoryProfile::new();
        profile.record("heap", 1024 * 1024);

        let summary = profile.summary();
        assert_eq!(summary.peak_mb, 1);
    }
}
