//! Knowledge base backend implementations
//!
//! Provides implementations for CAS (Content-Addressed Store), UMS (Universal Module System),
//! and Dynamic State backends for fact verification.

pub mod cas_backend;
pub mod ums_backend;
pub mod dynamic_state_backend;

pub use cas_backend::CasKnowledgeBase;
pub use ums_backend::UmsKnowledgeBase;
pub use dynamic_state_backend::DynamicStateBackend;

use ahf_core::{FactualClaim, VerificationResult};
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for knowledge base backends
///
/// All implementations must be deterministic and thread-safe.
#[async_trait]
pub trait KnowledgeBackend: Send + Sync {
    /// Look up a single fact
    ///
    /// Returns VerificationResult with evidence if fact is found/verified.
    /// Returns Unknown if fact cannot be determined.
    /// Returns Contradicted if conflicting evidence is found.
    async fn lookup(&self, claim: &FactualClaim) -> crate::KgsResult<VerificationResult>;

    /// Look up multiple facts (batch operation)
    ///
    /// More efficient than sequential lookups.
    async fn batch_lookup(&self, claims: &[FactualClaim]) -> crate::KgsResult<Vec<VerificationResult>> {
        let mut results = Vec::with_capacity(claims.len());
        for claim in claims {
            results.push(self.lookup(claim).await?);
        }
        Ok(results)
    }

    /// Get the human-readable name of this backend
    fn name(&self) -> &str;

    /// Get reliability score (0.0 to 1.0)
    ///
    /// Higher scores indicate more trustworthy sources.
    /// Used for weighting evidence in aggregate scoring.
    fn reliability_score(&self) -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_backend_trait() {
        // Trait definition is correct
        let _: &dyn KnowledgeBackend;
    }
}
