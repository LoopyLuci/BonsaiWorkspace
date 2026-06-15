//! Metrics and monitoring for the AI fallback framework

use serde::{Serialize, Deserialize};
use crate::ExecutionTier;

/// Metrics for Arbiter execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArbiterMetrics {
    pub total_executions: u64,
    pub ai_successes: u64,
    pub heuristic_fallbacks: u64,
    pub core_fallbacks: u64,
    pub stub_fallbacks: u64,
    pub ai_latency_sum_us: u64,
    pub core_latency_sum_us: u64,
    pub ai_poisoned_count: u64,
}

impl ArbiterMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_execution(&mut self, tier: ExecutionTier) {
        self.total_executions += 1;
        match tier {
            ExecutionTier::AiEnhanced => self.ai_successes += 1,
            ExecutionTier::Heuristic => self.heuristic_fallbacks += 1,
            ExecutionTier::DeterministicCore => self.core_fallbacks += 1,
            ExecutionTier::SafeStub => self.stub_fallbacks += 1,
        }
    }

    pub fn record_ai_latency(&mut self, latency_us: u64) {
        self.ai_latency_sum_us += latency_us;
    }

    pub fn record_core_latency(&mut self, latency_us: u64) {
        self.core_latency_sum_us += latency_us;
    }

    pub fn record_poisoned_output(&mut self) {
        self.ai_poisoned_count += 1;
    }

    pub fn ai_success_rate(&self) -> f32 {
        if self.total_executions == 0 {
            return 0.0;
        }
        self.ai_successes as f32 / self.total_executions as f32
    }

    pub fn avg_ai_latency_us(&self) -> u64 {
        if self.ai_successes == 0 {
            return 0;
        }
        self.ai_latency_sum_us / self.ai_successes
    }

    pub fn avg_core_latency_us(&self) -> u64 {
        if self.core_fallbacks == 0 {
            return 0;
        }
        self.core_latency_sum_us / self.core_fallbacks
    }
}

/// State of an Arbiter instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArbiterState {
    /// Operating normally with all tiers available
    Operational,
    /// AI domain is degraded or slow
    AiDegraded,
    /// AI domain is in quarantine (flaky)
    AiQuarantined,
    /// All AI disabled, running deterministic only
    DeterministicOnly,
    /// System is in safe mode (only stubs)
    SafeMode,
}

impl ArbiterState {
    pub fn is_degraded(&self) -> bool {
        matches!(
            self,
            ArbiterState::AiDegraded | ArbiterState::AiQuarantined | ArbiterState::SafeMode
        )
    }

    pub fn ai_available(&self) -> bool {
        matches!(self, ArbiterState::Operational | ArbiterState::AiDegraded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_tracking() {
        let mut metrics = ArbiterMetrics::new();
        metrics.record_execution(ExecutionTier::AiEnhanced);
        metrics.record_execution(ExecutionTier::AiEnhanced);
        metrics.record_execution(ExecutionTier::DeterministicCore);

        assert_eq!(metrics.total_executions, 3);
        assert_eq!(metrics.ai_successes, 2);
        assert_eq!(metrics.core_fallbacks, 1);
        assert_eq!(metrics.ai_success_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_arbiter_state() {
        let state = ArbiterState::Operational;
        assert!(!state.is_degraded());
        assert!(state.ai_available());

        let degraded = ArbiterState::AiQuarantined;
        assert!(degraded.is_degraded());
        assert!(!degraded.ai_available());
    }
}
