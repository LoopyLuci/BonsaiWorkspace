use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// SLA target configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLATarget {
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub availability_percent: f64,
}

/// SLA observation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAObservation {
    pub operation: String,
    pub latency_ms: f64,
    pub success: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// SLA compliance status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLACompliance {
    pub p95_met: bool,
    pub p99_met: bool,
    pub availability_met: bool,
    pub current_p95_ms: f64,
    pub current_p99_ms: f64,
    pub current_availability_percent: f64,
    pub compliance_percent: f64,
}

/// Tracks SLA compliance
pub struct SLATracker {
    target: SLATarget,
    observations: Arc<RwLock<Vec<SLAObservation>>>,
}

impl SLATracker {
    pub fn new(target: SLATarget) -> Self {
        Self {
            target,
            observations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record an observation
    pub fn record(&self, operation: &str, latency_ms: f64, success: bool) {
        let mut obs = self.observations.write();
        obs.push(SLAObservation {
            operation: operation.to_string(),
            latency_ms,
            success,
            timestamp: chrono::Utc::now(),
        });

        // Keep only last 10000 observations (1 hour of data at 3 ops/sec avg)
        if obs.len() > 10000 {
            obs.drain(0..1000);
        }
    }

    /// Calculate current SLA compliance
    pub fn get_compliance(&self) -> SLACompliance {
        let obs = self.observations.read();

        if obs.is_empty() {
            return SLACompliance {
                p95_met: true,
                p99_met: true,
                availability_met: true,
                current_p95_ms: 0.0,
                current_p99_ms: 0.0,
                current_availability_percent: 100.0,
                compliance_percent: 100.0,
            };
        }

        // Calculate percentiles
        let mut latencies: Vec<f64> = obs.iter().map(|o| o.latency_ms).collect();
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p95_idx = ((latencies.len() as f64) * 0.95) as usize;
        let p99_idx = ((latencies.len() as f64) * 0.99) as usize;

        let current_p95 = latencies.get(p95_idx).copied().unwrap_or(0.0);
        let current_p99 = latencies.get(p99_idx).copied().unwrap_or(0.0);

        // Calculate availability
        let successes = obs.iter().filter(|o| o.success).count() as f64;
        let total = obs.len() as f64;
        let current_availability = (successes / total) * 100.0;

        // Check if SLA targets met
        let p95_met = current_p95 <= self.target.p95_latency_ms;
        let p99_met = current_p99 <= self.target.p99_latency_ms;
        let availability_met = current_availability >= self.target.availability_percent;

        // Calculate overall compliance
        let compliance_percent = if p95_met && p99_met && availability_met {
            100.0
        } else {
            let mut score = 100.0;
            if !p95_met {
                score -= 20.0;
            }
            if !p99_met {
                score -= 20.0;
            }
            if !availability_met {
                let diff = self.target.availability_percent - current_availability;
                score -= (diff * 0.5).min(20.0);
            }
            score.max(0.0)
        };

        SLACompliance {
            p95_met,
            p99_met,
            availability_met,
            current_p95_ms: current_p95,
            current_p99_ms: current_p99,
            current_availability_percent: current_availability,
            compliance_percent,
        }
    }

    /// Get compliance for specific operation
    pub fn get_operation_compliance(&self, operation: &str) -> SLACompliance {
        let obs = self.observations.read();
        let op_obs: Vec<_> = obs.iter().filter(|o| o.operation == operation).collect();

        if op_obs.is_empty() {
            return SLACompliance {
                p95_met: true,
                p99_met: true,
                availability_met: true,
                current_p95_ms: 0.0,
                current_p99_ms: 0.0,
                current_availability_percent: 100.0,
                compliance_percent: 100.0,
            };
        }

        // Same calculation as overall compliance
        let mut latencies: Vec<f64> = op_obs.iter().map(|o| o.latency_ms).collect();
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p95_idx = ((latencies.len() as f64) * 0.95) as usize;
        let p99_idx = ((latencies.len() as f64) * 0.99) as usize;

        let current_p95 = latencies.get(p95_idx).copied().unwrap_or(0.0);
        let current_p99 = latencies.get(p99_idx).copied().unwrap_or(0.0);

        let successes = op_obs.iter().filter(|o| o.success).count() as f64;
        let total = op_obs.len() as f64;
        let current_availability = (successes / total) * 100.0;

        let p95_met = current_p95 <= self.target.p95_latency_ms;
        let p99_met = current_p99 <= self.target.p99_latency_ms;
        let availability_met = current_availability >= self.target.availability_percent;

        let compliance_percent = if p95_met && p99_met && availability_met {
            100.0
        } else {
            let mut score = 100.0;
            if !p95_met {
                score -= 20.0;
            }
            if !p99_met {
                score -= 20.0;
            }
            if !availability_met {
                let diff = self.target.availability_percent - current_availability;
                score -= (diff * 0.5).min(20.0);
            }
            score.max(0.0)
        };

        SLACompliance {
            p95_met,
            p99_met,
            availability_met,
            current_p95_ms: current_p95,
            current_p99_ms: current_p99,
            current_availability_percent: current_availability,
            compliance_percent,
        }
    }

    /// Get all observations
    pub fn get_observations(&self) -> Vec<SLAObservation> {
        self.observations.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sla_compliance() {
        let target = SLATarget {
            p95_latency_ms: 100.0,
            p99_latency_ms: 200.0,
            availability_percent: 99.95,
        };
        let tracker = SLATracker::new(target);

        // Add some observations
        for i in 0..100 {
            tracker.record("test_op", (i as f64) * 0.5, true);
        }

        let compliance = tracker.get_compliance();
        assert!(compliance.compliance_percent > 0.0);
    }
}
