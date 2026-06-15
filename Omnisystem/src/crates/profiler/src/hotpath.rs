use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Detects and tracks hot (slow) execution paths
pub struct HotPathDetector {
    paths: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    threshold_ms: f64,
}

impl HotPathDetector {
    pub fn new() -> Self {
        Self {
            paths: Arc::new(RwLock::new(HashMap::new())),
            threshold_ms: 10.0,  // Operations >10ms are considered hot
        }
    }

    /// Record a path execution time
    pub fn record(&self, path: &str, duration_ms: f64) -> bool {
        let is_hot = duration_ms > self.threshold_ms;
        if is_hot {
            let mut paths = self.paths.write();
            paths
                .entry(path.to_string())
                .or_insert_with(Vec::new)
                .push(duration_ms);
        }
        is_hot
    }

    /// Get hottest paths ranked by total time
    pub fn get_hottest_paths(&self, limit: usize) -> Vec<(String, f64)> {
        let paths = self.paths.read();
        let mut ranked: Vec<_> = paths
            .iter()
            .map(|(path, times)| (path.clone(), times.iter().sum::<f64>()))
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        ranked.truncate(limit);
        ranked
    }

    /// Check if path is considered hot
    pub fn is_hot_path(&self, path: &str) -> bool {
        self.paths.read().contains_key(path)
    }

    /// Get average duration for hot path
    pub fn avg_duration_ms(&self, path: &str) -> Option<f64> {
        self.paths
            .read()
            .get(path)
            .map(|times| times.iter().sum::<f64>() / times.len() as f64)
    }
}

impl Default for HotPathDetector {
    fn default() -> Self {
        Self::new()
    }
}
