//! Knowledge Grounding Service (KGS)
//!
//! Main service that orchestrates fact extraction, lookup, and scoring.

use crate::backends::KnowledgeBackend;
use crate::cache::Cache;
use crate::extraction::{FactExtractor, ExtractionResult};
use crate::scoring::GroundingScorer;
use ahf_core::{FactualClaim, GroundingScore, VerificationResult};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Knowledge Grounding Service
///
/// Orchestrates the complete grounding pipeline:
/// 1. Extract facts from text
/// 2. Lookup facts in multiple knowledge bases
/// 3. Calculate grounding scores
/// 4. Return verification results
pub struct KnowledgeGroundingService {
    /// Fact extractor (deterministic)
    extractor: FactExtractor,
    /// Knowledge base backends
    backends: Vec<Arc<dyn KnowledgeBackend>>,
    /// Grounding scorer
    scorer: GroundingScorer,
    /// Result cache
    cache: Cache,
}

/// Configuration for KGS
pub struct KgsConfig {
    /// Default cache TTL
    pub cache_ttl: Duration,
    /// Maximum lookup timeout per backend
    pub lookup_timeout: Duration,
}

impl Default for KgsConfig {
    fn default() -> Self {
        KgsConfig {
            cache_ttl: Duration::from_secs(300),
            lookup_timeout: Duration::from_millis(100),
        }
    }
}

impl KnowledgeGroundingService {
    /// Create a new service with default configuration
    pub fn new(backends: Vec<Arc<dyn KnowledgeBackend>>) -> Self {
        KnowledgeGroundingService {
            extractor: FactExtractor::new(),
            backends,
            scorer: GroundingScorer::new(),
            cache: Cache::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(backends: Vec<Arc<dyn KnowledgeBackend>>, config: KgsConfig) -> Self {
        KnowledgeGroundingService {
            extractor: FactExtractor::new(),
            backends,
            scorer: GroundingScorer::new(),
            cache: Cache::with_ttl(config.cache_ttl),
        }
    }

    /// Extract facts from text
    pub fn extract_facts(&self, text: &str) -> crate::KgsResult<ExtractionResult> {
        info!("Extracting facts from text: {} bytes", text.len());
        self.extractor.extract(text)
    }

    /// Lookup a single fact across all backends
    pub async fn lookup(&self, claim: &FactualClaim) -> crate::KgsResult<VerificationResult> {
        let cache_key = Cache::make_key(&claim.object);

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!("Cache hit for: {}", claim.object);
            return Ok(cached);
        }

        // Try each backend in sequence
        for backend in &self.backends {
            debug!(
                "Looking up claim in backend: {}",
                backend.name()
            );

            match backend.lookup(claim).await {
                Ok(result) => {
                    // Cache successful lookups
                    self.cache.insert(cache_key.clone(), result.clone());
                    return Ok(result);
                }
                Err(e) => {
                    debug!("Lookup failed in backend {}: {}", backend.name(), e);
                    // Continue to next backend
                    continue;
                }
            }
        }

        // Not found in any backend
        Ok(VerificationResult {
            status: ahf_core::VerificationStatus::Inconclusive,
            proof: None,
            reasoning: "Claim not found in any knowledge base".to_string(),
            confidence: 0.0,
        })
    }

    /// Lookup multiple facts (batch operation)
    pub async fn lookup_batch(
        &self,
        claims: &[FactualClaim],
    ) -> crate::KgsResult<Vec<VerificationResult>> {
        info!("Looking up {} claims", claims.len());
        let mut results = Vec::with_capacity(claims.len());

        for claim in claims {
            let result = self.lookup(claim).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Calculate grounding score from verification results
    pub fn calculate_score(&self, results: &[VerificationResult]) -> GroundingScore {
        debug!("Calculating grounding score for {} results", results.len());
        self.scorer.score_batch(results)
    }

    /// Complete pipeline: extract, lookup, score
    pub async fn ground_text(&self, text: &str) -> crate::KgsResult<GroundingScore> {
        info!("Grounding text: {} bytes", text.len());

        // Step 1: Extract facts
        let extraction = self.extract_facts(text)?;
        debug!(
            "Extracted {} claims with average confidence: {:.2}",
            extraction.metrics.claims_count, extraction.metrics.avg_confidence
        );

        if extraction.claims.is_empty() {
            // No claims = fully grounded (nothing to verify)
            return Ok(GroundingScore::new(0, 0));
        }

        // Step 2: Lookup facts
        let results = self.lookup_batch(&extraction.claims).await?;

        // Step 3: Calculate score
        let score = self.calculate_score(&results);
        info!(
            "Grounding score: {:.2}",
            score.score()
        );

        Ok(score)
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Clean up expired cache entries
    pub fn cleanup_cache(&self) {
        self.cache.cleanup_expired();
    }

    /// Get registered backends
    pub fn backends(&self) -> Vec<&dyn KnowledgeBackend> {
        self.backends
            .iter()
            .map(|b| b.as_ref())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{CasKnowledgeBase, UmsKnowledgeBase};

    #[test]
    fn test_kgs_creation() {
        let backends: Vec<Arc<dyn KnowledgeBackend>> = vec![
            Arc::new(CasKnowledgeBase::new()),
            Arc::new(UmsKnowledgeBase::new()),
        ];

        let service = KnowledgeGroundingService::new(backends);
        assert_eq!(service.backends.len(), 2);
    }

    #[test]
    fn test_extract_facts() {
        let backends: Vec<Arc<dyn KnowledgeBackend>> =
            vec![Arc::new(CasKnowledgeBase::new())];

        let service = KnowledgeGroundingService::new(backends);
        let result = service.extract_facts("Paris is the capital of France.").unwrap();

        assert!(result.claims.len() > 0);
        assert!(result.metrics.avg_confidence > 0.0);
    }

    #[tokio::test]
    async fn test_ground_text() {
        let backends: Vec<Arc<dyn KnowledgeBackend>> =
            vec![Arc::new(CasKnowledgeBase::new())];

        let service = KnowledgeGroundingService::new(backends);
        let score = service.ground_text("Paris is the capital of France.").await.unwrap();

        assert!(score.score() >= 0.0 && score.score() <= 1.0);
    }

    #[tokio::test]
    async fn test_lookup_batch() {
        use ahf_core::{Subject, Predicate};
        use chrono::Utc;

        let backends: Vec<Arc<dyn KnowledgeBackend>> =
            vec![Arc::new(CasKnowledgeBase::new())];

        let service = KnowledgeGroundingService::new(backends);

        let claims = vec![
            FactualClaim {
                id: uuid::Uuid::new_v4(),
                subject: Subject::new("paris", "Paris"),
                predicate: Predicate::new("is_capital_of", "is_capital_of"),
                object: "France".to_string(),
                context: None,
                source_confidence: 0.9,
                timestamp: Utc::now(),
                source_reference: None,
            },
        ];

        let results = service.lookup_batch(&claims).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_cache_stats() {
        let backends: Vec<Arc<dyn KnowledgeBackend>> =
            vec![Arc::new(CasKnowledgeBase::new())];

        let service = KnowledgeGroundingService::new(backends);
        let stats = service.cache_stats();

        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_kgs_with_config() {
        let backends: Vec<Arc<dyn KnowledgeBackend>> =
            vec![Arc::new(CasKnowledgeBase::new())];

        let config = KgsConfig {
            cache_ttl: Duration::from_secs(60),
            lookup_timeout: Duration::from_millis(50),
        };

        let service = KnowledgeGroundingService::with_config(backends, config);
        assert_eq!(service.backends.len(), 1);
    }
}
