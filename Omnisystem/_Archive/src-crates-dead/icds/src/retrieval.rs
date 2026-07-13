//! Retrieval engine with resolution cascade
//!
//! Implements multi-resolution retrieval: Keywords → Summaries → Full Text

use crate::atom::{AtomId, EmbeddingVector, ResolutionLevel, SemanticAtom};
use crate::error::Result;
use crate::index::HierarchicalIndex;
use crate::storage::AtomStore;
use crate::IcdsConfig;
use parking_lot::RwLock;
use std::sync::Arc;

/// Query result with atoms at appropriate resolution level
#[derive(Clone, Debug)]
pub struct QueryResult {
    /// Retrieved atoms
    pub atoms: Vec<RetrievedAtom>,
    /// Query latency in microseconds
    pub latency_us: u64,
}

/// A single retrieved atom with its resolution
#[derive(Clone, Debug)]
pub struct RetrievedAtom {
    /// The atom
    pub atom: SemanticAtom,
    /// Similarity score
    pub score: f32,
    /// Resolution level returned
    pub resolution: ResolutionLevel,
}

/// LRU cache for hot atoms
struct HotCache {
    atoms: parking_lot::lock_api::RwLock<DashMapMutex, std::collections::VecDeque<AtomId>>,
    size: usize,
}

use dashmap::DashMap;

type DashMapMutex = std::sync::Mutex<()>;

impl HotCache {
    fn new(size: usize) -> Self {
        Self {
            atoms: parking_lot::lock_api::RwLock::new(DashMapMutex::default(), std::collections::VecDeque::new()),
            size,
        }
    }

    fn get(&self, id: &AtomId) -> bool {
        // Simple implementation - just check if in VecDeque
        false // Placeholder
    }

    fn put(&self, id: AtomId) {
        // Simple LRU replacement
    }
}

/// Retrieval engine implementing resolution cascade
pub struct RetrievalEngine {
    store: Arc<dyn AtomStore>,
    index: Arc<HierarchicalIndex>,
    config: IcdsConfig,
    hot_cache: Arc<RwLock<std::collections::VecDeque<AtomId>>>,
    cache_hits: Arc<RwLock<u64>>,
    cache_total: Arc<RwLock<u64>>,
}

impl RetrievalEngine {
    /// Create a new retrieval engine
    pub fn new(
        store: Arc<dyn AtomStore>,
        index: Arc<HierarchicalIndex>,
        config: IcdsConfig,
    ) -> Self {
        Self {
            store,
            index,
            config,
            hot_cache: Arc::new(RwLock::new(std::collections::VecDeque::new())),
            cache_hits: Arc::new(RwLock::new(0)),
            cache_total: Arc::new(RwLock::new(0)),
        }
    }

    /// Execute a query with resolution cascade
    ///
    /// Returns results in progressively higher resolution:
    /// 1. Keywords (Level 2) for initial scoring
    /// 2. Summaries (Level 1) for top candidates
    /// 3. Full text (Level 0) for final results
    pub async fn query(&self, text: &str, limit: usize) -> Result<QueryResult> {
        let start = std::time::Instant::now();

        // Generate query embedding
        let query_embedding = EmbeddingVector::sparse_from_text(text);

        // Step 1: Search index (returns all candidates initially at keywords level)
        let candidates = self.index.search(&query_embedding, limit * 2).await?;

        let mut results = Vec::new();

        // Step 2: For top candidates, fetch full atoms and determine resolution
        for (atom_id, score) in candidates.iter().take(limit) {
            // Increment cache stats
            *self.cache_total.write() += 1;

            // Check hot cache first
            if self.is_in_cache(atom_id).await {
                *self.cache_hits.write() += 1;
            }

            match self.store.get(atom_id).await? {
                Some(atom) => {
                    // Determine which resolution level to return
                    // For now: return full text if available
                    let resolution = ResolutionLevel::Full;

                    results.push(RetrievedAtom {
                        atom,
                        score: *score,
                        resolution,
                    });
                }
                None => {
                    // Atom was deleted or not found
                    continue;
                }
            }
        }

        let latency_us = start.elapsed().as_micros() as u64;

        Ok(QueryResult {
            atoms: results,
            latency_us,
        })
    }

    /// Check if atom is in hot cache
    async fn is_in_cache(&self, atom_id: &AtomId) -> bool {
        let cache = self.hot_cache.read();
        cache.contains(atom_id)
    }

    /// Add atom to hot cache (LRU)
    async fn cache_put(&self, atom_id: AtomId) {
        let mut cache = self.hot_cache.write();

        if cache.len() >= self.config.hot_cache_size {
            cache.pop_front(); // Evict oldest
        }

        cache.push_back(atom_id);
    }

    /// Get cache hit rate
    pub async fn cache_hit_rate(&self) -> f64 {
        let hits = *self.cache_hits.read();
        let total = *self.cache_total.read();

        if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::{AtomMetadata, SourceType};
    use crate::storage::MemoryAtomStore;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_retrieval_engine_query() {
        let store = Arc::new(MemoryAtomStore::new());
        let index = Arc::new(HierarchicalIndex::new(16, 200).unwrap());
        let config = IcdsConfig::default();
        let engine = RetrievalEngine::new(store.clone(), index.clone(), config);

        let atom = SemanticAtom::from_text(
            "hello world test".to_string(),
            AtomMetadata {
                source: SourceType::UserInput,
                agent_id: Uuid::nil(),
                conversation_id: None,
                tags: vec![],
                importance: 1.0,
            },
            3,
        )
        .unwrap();

        store.store(&atom).await.unwrap();
        index.insert(&atom).await.unwrap();

        let results = engine.query("hello world", 10).await.unwrap();
        assert!(!results.atoms.is_empty());
    }

    #[tokio::test]
    async fn test_cache_hit_rate() {
        let store = Arc::new(MemoryAtomStore::new());
        let index = Arc::new(HierarchicalIndex::new(16, 200).unwrap());
        let config = IcdsConfig::default();
        let engine = RetrievalEngine::new(store, index, config);

        assert_eq!(engine.cache_hit_rate().await, 0.0);
    }
}
