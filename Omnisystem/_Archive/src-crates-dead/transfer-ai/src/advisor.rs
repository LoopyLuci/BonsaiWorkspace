//! AI Congestion Control Advisor – Advisory only, never modifies core state directly

use serde::{Serialize, Deserialize};

/// An AI-powered congestion advisor that provides suggestions (not commands).
/// Runs in its own context and communicates via channels only.
/// All suggestions are validated against SafetyEnvelope bounds before use.
#[derive(Debug, Clone)]
pub struct AiCongestionAdvisor {
    is_healthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAdvice {
    pub suggested_cwnd: u64,
    pub suggested_pacing_rate: u64,
    pub confidence: f32,
    pub reasoning: String,
}

impl AiCongestionAdvisor {
    pub fn new(_confidence_threshold: f32) -> Self {
        Self {
            is_healthy: false,
        }
    }

    /// Load a pre-trained model (optional — if not loaded, advisor is inactive).
    pub async fn load_model(&mut self, _path: &str) -> anyhow::Result<()> {
        self.is_healthy = true;
        Ok(())
    }

    /// Unload the model — advisor becomes inactive, all connections fall back to deterministic control.
    pub fn unload_model(&mut self) {
        self.is_healthy = false;
    }

    /// Provide an advisory suggestion. Returns `None` if the model is unavailable or confidence is below threshold.
    pub async fn advise(&self, _rtt_ms: f32, _loss_rate: f32) -> Option<AiAdvice> {
        if !self.is_healthy {
            return None;
        }
        // Placeholder: in production, this would call a neural network model.
        // For now, return None (advisor inactive).
        None
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advisor_inactive_by_default() {
        let advisor = AiCongestionAdvisor::new(0.9);
        assert!(!advisor.is_healthy());
    }

    #[tokio::test]
    async fn test_advisor_activate() {
        let mut advisor = AiCongestionAdvisor::new(0.9);
        advisor.load_model("dummy").await.unwrap();
        assert!(advisor.is_healthy());
    }
}
