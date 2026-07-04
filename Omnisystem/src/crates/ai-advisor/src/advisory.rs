//! Advisory domain output and health monitoring

use alloc::vec::Vec;
use serde::{Serialize, Deserialize};

/// Output from the AI advisory domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisoryOutput {
    /// The suggested data/action
    pub data: Vec<u8>,
    /// Model confidence (0.0 – 1.0)
    pub confidence: f32,
    /// Latency of this suggestion in microseconds
    pub latency_us: u64,
    /// Hash of the model that produced this output
    pub model_hash: [u8; 32],
}

impl AdvisoryOutput {
    pub fn new(data: Vec<u8>, confidence: f32, latency_us: u64) -> Self {
        Self {
            data,
            confidence,
            latency_us,
            model_hash: [0u8; 32],
        }
    }

    pub fn with_model_hash(mut self, hash: [u8; 32]) -> Self {
        self.model_hash = hash;
        self
    }

    pub fn is_confident(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }

    pub fn is_timely(&self, deadline_us: u64) -> bool {
        self.latency_us < deadline_us
    }
}

/// Health status of the AI advisory domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryHealth {
    /// Advisory domain is operational
    Healthy,
    /// Advisory domain is responding but may be degraded
    Degraded,
    /// Advisory domain is unhealthy or crashed
    Unhealthy,
    /// Advisory domain is in quarantine (flaky)
    Quarantined,
}

/// Tracks recent advice for consistency checking
#[derive(Debug, Clone)]
pub struct ConsistencyWindow {
    pub recent_advice: Vec<AdvisoryOutput>,
    pub max_size: usize,
}

impl ConsistencyWindow {
    pub fn new(max_size: usize) -> Self {
        Self {
            recent_advice: Vec::new(),
            max_size,
        }
    }

    pub fn push(&mut self, advice: AdvisoryOutput) {
        self.recent_advice.push(advice);
        if self.recent_advice.len() > self.max_size {
            self.recent_advice.remove(0);
        }
    }

    pub fn is_consistent(&self, epsilon: f32) -> bool {
        if self.recent_advice.len() < 2 {
            return false;
        }

        let first = &self.recent_advice[0].data.len() as f32;
        let mut min = first;
        let mut max = first;

        for advice in &self.recent_advice[1..] {
            let size = advice.data.len() as f32;
            if size < min {
                min = size;
            }
            if size > max {
                max = size;
            }
        }

        let deviation = (max - min) / first.max(1.0);
        deviation <= epsilon
    }
}

/// Trait for implementing custom advisory domains
pub trait AdvisoryDomain: Send + Sync {
    /// Get the next advisory output
    fn suggest(&mut self, input: &[u8]) -> Option<AdvisoryOutput>;

    /// Get the health status
    fn health(&self) -> AdvisoryHealth;

    /// Reset the advisory domain (after failure or degradation)
    fn reset(&mut self);

    /// Get the model hash
    fn model_hash(&self) -> [u8; 32];
}

/// A no-op advisory domain that is always disabled
pub struct DisabledAdvisory;

impl AdvisoryDomain for DisabledAdvisory {
    fn suggest(&mut self, _input: &[u8]) -> Option<AdvisoryOutput> {
        None
    }

    fn health(&self) -> AdvisoryHealth {
        AdvisoryHealth::Unhealthy
    }

    fn reset(&mut self) {}

    fn model_hash(&self) -> [u8; 32] {
        [0u8; 32]
    }
}
