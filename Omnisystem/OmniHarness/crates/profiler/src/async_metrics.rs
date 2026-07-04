use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Tracks async operation metrics
pub struct AsyncMetrics {
    operations: Arc<RwLock<HashMap<String, Vec<f64>>>>,
}

impl AsyncMetrics {
    pub fn new() -> Self {
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record an async operation duration
    pub fn record(&self, operation: &str, duration_secs: f64) {
        let mut ops = self.operations.write();
        ops.entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration_secs * 1000.0);  // Convert to ms
    }

    /// Get average duration for operation
    pub fn avg_duration_ms(&self, operation: &str) -> Option<f64> {
        self.operations
            .read()
            .get(operation)
            .map(|durations| durations.iter().sum::<f64>() / durations.len() as f64)
    }

    /// Get all recorded operations
    pub fn get_all(&self) -> HashMap<String, f64> {
        self.operations
            .read()
            .iter()
            .map(|(op, durations)| {
                (
                    op.clone(),
                    durations.iter().sum::<f64>() / durations.len() as f64,
                )
            })
            .collect()
    }
}

impl Default for AsyncMetrics {
    fn default() -> Self {
        Self::new()
    }
}
