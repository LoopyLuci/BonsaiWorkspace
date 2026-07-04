//! Core service trait and execution tier definitions

use alloc::vec::Vec;
use serde::{Serialize, Deserialize};
use crate::{Error, Result, AdvisoryOutput};

/// Execution tier in the graceful degradation ladder
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionTier {
    /// AI-enhanced execution (fast, adaptive, optional)
    AiEnhanced,
    /// Rule-based heuristic fallback (deterministic, simple)
    Heuristic,
    /// Deterministic core (slow but correct, always available)
    DeterministicCore,
    /// Safe stub (minimal functionality, never fails)
    SafeStub,
}

impl ExecutionTier {
    pub fn level(&self) -> u8 {
        match self {
            ExecutionTier::AiEnhanced => 3,
            ExecutionTier::Heuristic => 2,
            ExecutionTier::DeterministicCore => 1,
            ExecutionTier::SafeStub => 0,
        }
    }
}

/// Result of executing a service with tier information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub data: Vec<u8>,
    pub tier: ExecutionTier,
    pub confidence: f32,
}

/// Every Bonsai service must implement this trait
pub trait SovereignService {
    /// Deterministic core: pure algorithms, no ML, formally verified.
    /// Must always succeed or return a safe stub result.
    /// This is the primary operational mode.
    fn deterministic_core(&self, input: &[u8]) -> Result<Vec<u8>>;

    /// Heuristic layer: rule-based fallback.
    /// Optional if the core covers all cases.
    /// Returns None if not applicable.
    fn heuristic(&self, input: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    /// Advisory domain: optional AI/ML enhancement.
    /// Returns None if AI is disabled or unavailable.
    /// Must be sandboxed; errors are treated as "AI failure" not service failure.
    fn ai_suggestion(&self, input: &[u8]) -> Option<AdvisoryOutput> {
        None
    }

    /// Safe stub: minimal functionality that never fails.
    /// Ensures graceful degradation to a usable state.
    fn safe_stub(&self, _input: &[u8]) -> Vec<u8> {
        Vec::new()
    }

    /// Get a human-readable name for this service.
    fn name(&self) -> &str {
        "UnnamedService"
    }
}

/// Configuration for deterministic core execution
#[derive(Debug, Clone)]
pub struct DeterministicCore {
    pub always_enabled: bool,
    pub timeout_us: u64,
}

impl Default for DeterministicCore {
    fn default() -> Self {
        Self {
            always_enabled: true,
            timeout_us: 10_000, // 10ms
        }
    }
}

/// Configuration for heuristic layer
#[derive(Debug, Clone)]
pub struct HeuristicLayer {
    pub enabled: bool,
    pub timeout_us: u64,
}

impl Default for HeuristicLayer {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_us: 5_000, // 5ms
        }
    }
}
