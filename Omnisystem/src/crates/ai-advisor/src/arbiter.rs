//! Trusted Arbiter – orchestrates graceful degradation ladder

use alloc::vec::Vec;
use alloc::collections::VecDeque;
use serde::{Serialize, Deserialize};
use crate::{
    Error, Result, SovereignService, ExecutionTier, ExecutionResult, AdvisoryOutput,
    AdvisoryHealth, ConsistencyWindow,
};

/// Configuration for the Trusted Arbiter
#[derive(Debug, Clone)]
pub struct ArbiterConfig {
    /// Enable AI domain
    pub ai_enabled: bool,
    /// Minimum confidence threshold for AI advice (0.0 – 1.0)
    pub min_confidence: f32,
    /// Maximum latency budget for AI (microseconds)
    pub ai_latency_limit_us: u64,
    /// Consistency epsilon: max deviation before AI is marked flaky
    pub consistency_epsilon: f32,
    /// Size of consistency window
    pub consistency_window_size: usize,
    /// Enable heuristic fallback
    pub heuristic_enabled: bool,
}

impl Default for ArbiterConfig {
    fn default() -> Self {
        Self {
            ai_enabled: false, // Disabled by default for security
            min_confidence: 0.9,
            ai_latency_limit_us: 5_000, // 5ms
            consistency_epsilon: 0.1,
            consistency_window_size: 8,
            heuristic_enabled: true,
        }
    }
}

/// Decision made by the Trusted Arbiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDecision {
    pub tier: ExecutionTier,
    pub reason: String,
    pub confidence: f32,
}

impl ExecutionDecision {
    pub fn ai_enhanced(reason: &str) -> Self {
        Self {
            tier: ExecutionTier::AiEnhanced,
            reason: reason.into(),
            confidence: 1.0,
        }
    }

    pub fn heuristic(reason: &str) -> Self {
        Self {
            tier: ExecutionTier::Heuristic,
            reason: reason.into(),
            confidence: 0.8,
        }
    }

    pub fn deterministic_core(reason: &str) -> Self {
        Self {
            tier: ExecutionTier::DeterministicCore,
            reason: reason.into(),
            confidence: 0.6,
        }
    }

    pub fn safe_stub(reason: &str) -> Self {
        Self {
            tier: ExecutionTier::SafeStub,
            reason: reason.into(),
            confidence: 0.0,
        }
    }
}

/// The Trusted Arbiter – coordinates all execution decisions
pub struct Arbiter {
    config: ArbiterConfig,
    consistency_window: ConsistencyWindow,
    ai_health: AdvisoryHealth,
    recent_decisions: VecDeque<ExecutionDecision>,
    max_decisions_to_log: usize,
}

impl Arbiter {
    pub fn new(config: ArbiterConfig) -> Self {
        Self {
            consistency_window: ConsistencyWindow::new(config.consistency_window_size),
            ai_health: if config.ai_enabled {
                AdvisoryHealth::Healthy
            } else {
                AdvisoryHealth::Unhealthy
            },
            config,
            recent_decisions: VecDeque::new(),
            max_decisions_to_log: 100,
        }
    }

    /// Execute a service through the graceful degradation ladder
    pub fn execute(&mut self, service: &dyn SovereignService, input: &[u8]) -> ExecutionResult {
        // Step 1: Try AI if enabled and healthy
        if self.config.ai_enabled && self.ai_health == AdvisoryHealth::Healthy {
            if let Some(advice) = service.ai_suggestion(input) {
                if self.validate_ai_advice(&advice) {
                    self.consistency_window.push(advice.clone());
                    let decision = ExecutionDecision::ai_enhanced("AI provided high-confidence advice");
                    self.log_decision(decision.clone());
                    return ExecutionResult {
                        data: advice.data,
                        tier: ExecutionTier::AiEnhanced,
                        confidence: advice.confidence,
                    };
                }
            }
        }

        // Step 2: Try heuristic
        if self.config.heuristic_enabled {
            if let Ok(Some(result)) = service.heuristic(input) {
                let decision = ExecutionDecision::heuristic("Heuristic fallback");
                self.log_decision(decision.clone());
                return ExecutionResult {
                    data: result,
                    tier: ExecutionTier::Heuristic,
                    confidence: 0.8,
                };
            }
        }

        // Step 3: Deterministic core (always available)
        if let Ok(result) = service.deterministic_core(input) {
            let decision = ExecutionDecision::deterministic_core("Deterministic core fallback");
            self.log_decision(decision.clone());
            return ExecutionResult {
                data: result,
                tier: ExecutionTier::DeterministicCore,
                confidence: 0.6,
            };
        }

        // Step 4: Safe stub (never fails)
        let decision = ExecutionDecision::safe_stub("All tiers exhausted, using safe stub");
        self.log_decision(decision.clone());
        ExecutionResult {
            data: service.safe_stub(input),
            tier: ExecutionTier::SafeStub,
            confidence: 0.0,
        }
    }

    /// Validate AI advice against safety constraints
    fn validate_ai_advice(&mut self, advice: &AdvisoryOutput) -> bool {
        // Check latency
        if !advice.is_timely(self.config.ai_latency_limit_us) {
            self.ai_health = AdvisoryHealth::Degraded;
            return false;
        }

        // Check confidence
        if !advice.is_confident(self.config.min_confidence) {
            return false;
        }

        // Check consistency
        if !self.consistency_window.is_consistent(self.config.consistency_epsilon) {
            self.ai_health = AdvisoryHealth::Quarantined;
            return false;
        }

        true
    }

    /// Log an execution decision to Universe (stub for now)
    fn log_decision(&mut self, decision: ExecutionDecision) {
        self.recent_decisions.push_back(decision);
        if self.recent_decisions.len() > self.max_decisions_to_log {
            self.recent_decisions.pop_front();
        }
        // TODO: In production, send to Universe for immutable audit log
    }

    /// Get recent execution decisions
    pub fn recent_decisions(&self) -> Vec<ExecutionDecision> {
        self.recent_decisions.iter().cloned().collect()
    }

    /// Set AI health status
    pub fn set_ai_health(&mut self, health: AdvisoryHealth) {
        self.ai_health = health;
    }

    /// Get current AI health
    pub fn ai_health(&self) -> AdvisoryHealth {
        self.ai_health
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbiter_defaults_to_deterministic() {
        let config = ArbiterConfig::default();
        assert!(!config.ai_enabled);
    }

    #[test]
    fn test_execution_decision_creation() {
        let ai = ExecutionDecision::ai_enhanced("test");
        assert_eq!(ai.tier, ExecutionTier::AiEnhanced);

        let heuristic = ExecutionDecision::heuristic("test");
        assert_eq!(heuristic.tier, ExecutionTier::Heuristic);

        let core = ExecutionDecision::deterministic_core("test");
        assert_eq!(core.tier, ExecutionTier::DeterministicCore);

        let stub = ExecutionDecision::safe_stub("test");
        assert_eq!(stub.tier, ExecutionTier::SafeStub);
    }
}
