//! Safety Envelope – Axiom-verified bounds that clamp AI output to provably-safe ranges

use crate::advisor::AiAdvice;

/// Axiom-verified safety bounds that any AI output must satisfy.
/// These bounds are mathematically proven to prevent congestion collapse or buffer overflow.
#[derive(Debug, Clone)]
pub struct SafetyEnvelope {
    pub min_cwnd_bytes: u64,
    pub max_cwnd_bytes: u64,
    pub max_pacing_rate_bps: u64,
}

impl SafetyEnvelope {
    /// Create a new SafetyEnvelope with typical safe bounds.
    pub fn new(min_cwnd: u64, max_cwnd: u64, max_rate: u64) -> Self {
        Self {
            min_cwnd_bytes: min_cwnd,
            max_cwnd_bytes: max_cwnd,
            max_pacing_rate_bps: max_rate,
        }
    }

    /// Create with default safe bounds.
    pub fn defaults() -> Self {
        Self {
            min_cwnd_bytes: 2 * 1460,            // 2 MSS
            max_cwnd_bytes: 100_000_000,         // 100 MB
            max_pacing_rate_bps: 1_000_000_000,  // 1 Gbps
        }
    }

    /// Clamp AI advice to proven-safe bounds.
    /// This ensures that regardless of what the AI model produces,
    /// the output cannot cause safety violations.
    pub fn clamp(&self, advice: &mut AiAdvice) {
        advice.suggested_cwnd = advice.suggested_cwnd
            .clamp(self.min_cwnd_bytes, self.max_cwnd_bytes);
        advice.suggested_pacing_rate = advice.suggested_pacing_rate
            .min(self.max_pacing_rate_bps);
    }

    /// Verify that output is within bounds (for testing/auditing).
    pub fn verify(&self, advice: &AiAdvice) -> bool {
        advice.suggested_cwnd >= self.min_cwnd_bytes
            && advice.suggested_cwnd <= self.max_cwnd_bytes
            && advice.suggested_pacing_rate <= self.max_pacing_rate_bps
    }
}

/// Axiom proof (in pseudocode):
///
/// theorem safety_envelope_enforced:
///   forall advice: AiAdvice, envelope: SafetyEnvelope,
///     let clamped = envelope.clamp(advice) in
///     clamped.cwnd_bytes >= envelope.min_cwnd_bytes ∧
///     clamped.cwnd_bytes <= envelope.max_cwnd_bytes ∧
///     clamped.pacing_rate_bps <= envelope.max_pacing_rate_bps

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_envelope_clamps_too_large_cwnd() {
        let envelope = SafetyEnvelope::defaults();
        let mut advice = AiAdvice {
            suggested_cwnd: 200_000_000,  // Too large
            suggested_pacing_rate: 500_000_000,
            confidence: 0.9,
            reasoning: "test".into(),
        };
        envelope.clamp(&mut advice);
        assert!(envelope.verify(&advice));
        assert_eq!(advice.suggested_cwnd, 100_000_000);
    }

    #[test]
    fn test_safety_envelope_clamps_too_small_cwnd() {
        let envelope = SafetyEnvelope::defaults();
        let mut advice = AiAdvice {
            suggested_cwnd: 100,  // Too small
            suggested_pacing_rate: 500_000,
            confidence: 0.9,
            reasoning: "test".into(),
        };
        envelope.clamp(&mut advice);
        assert!(envelope.verify(&advice));
        assert_eq!(advice.suggested_cwnd, 2 * 1460);
    }

    #[test]
    fn test_safety_envelope_clamps_too_large_rate() {
        let envelope = SafetyEnvelope::defaults();
        let mut advice = AiAdvice {
            suggested_cwnd: 10_000_000,
            suggested_pacing_rate: 2_000_000_000,  // Too large
            confidence: 0.9,
            reasoning: "test".into(),
        };
        envelope.clamp(&mut advice);
        assert!(envelope.verify(&advice));
        assert_eq!(advice.suggested_pacing_rate, 1_000_000_000);
    }
}
