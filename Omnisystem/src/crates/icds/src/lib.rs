//! ICDS - Infinite Context Data Store
//!
//! A hierarchical, multi-resolution context management system for AI agents.
//! Provides content-addressed storage, a semantic vector index, and
//! resolution-cascade retrieval so that arbitrarily large conversation/task
//! histories can be queried and assembled into bounded-size context windows.

pub mod api;
pub mod atom;
pub mod context;
pub mod error;
pub mod index;
pub mod retrieval;
pub mod storage;
pub mod types;

use atom::{AtomId, AtomMetadata, SemanticAtom};
use error::Result;
use index::HierarchicalIndex;
use retrieval::{QueryResult, RetrievalEngine};
use std::sync::Arc;
use storage::{AtomStore, MemoryAtomStore};

/// Configuration for the Infinite Context Data Store engine
#[derive(Clone, Debug)]
pub struct IcdsConfig {
    /// Number of multi-resolution tiers to generate per atom
    pub num_resolutions: usize,
    /// HNSW-style index fan-out parameter
    pub index_m: usize,
    /// HNSW-style index construction parameter
    pub index_ef_construction: usize,
    /// Max number of atom IDs held in the hot cache
    pub hot_cache_size: usize,
}

impl Default for IcdsConfig {
    fn default() -> Self {
        Self {
            num_resolutions: 3,
            index_m: 16,
            index_ef_construction: 200,
            hot_cache_size: 256,
        }
    }
}

/// The top-level Infinite Context Data Store engine
///
/// Ties together atom storage, the semantic index, and the resolution-cascade
/// retrieval engine, and exposes ingest/query/assemble operations.
pub struct InfiniteContextEngine {
    store: Arc<dyn AtomStore>,
    index: Arc<HierarchicalIndex>,
    retrieval: RetrievalEngine,
    config: IcdsConfig,
}

impl InfiniteContextEngine {
    /// Create a new engine with default (in-memory) storage
    pub async fn new() -> Result<Self> {
        Self::with_config(IcdsConfig::default()).await
    }

    /// Create a new engine with custom configuration
    pub async fn with_config(config: IcdsConfig) -> Result<Self> {
        let store: Arc<dyn AtomStore> = Arc::new(MemoryAtomStore::new());
        let index = Arc::new(HierarchicalIndex::new(
            config.index_m,
            config.index_ef_construction,
        )?);
        let retrieval = RetrievalEngine::new(store.clone(), index.clone(), config.clone());

        Ok(Self {
            store,
            index,
            retrieval,
            config,
        })
    }

    /// Ingest raw text as a new semantic atom
    pub async fn ingest(&self, text: &str, metadata: AtomMetadata) -> Result<Vec<AtomId>> {
        let atom =
            SemanticAtom::from_text(text.to_string(), metadata, self.config.num_resolutions)?;
        let id = atom.id.clone();

        self.store.store(&atom).await?;
        self.index.insert(&atom).await?;

        Ok(vec![id])
    }

    /// Query the store, returning atoms ranked by relevance
    pub async fn query(&self, text: &str, limit: usize) -> Result<QueryResult> {
        self.retrieval.query(text, limit).await
    }

    /// Assemble a bounded-size context string for a given query
    pub async fn assemble_context(&self, query: &str, max_tokens: usize) -> Result<String> {
        let results = self.query(query, 20).await?;
        context::assemble_hierarchical(&results, max_tokens).await
    }

    /// Total number of atoms stored
    pub async fn atom_count(&self) -> Result<u64> {
        self.store.count().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_metadata() -> AtomMetadata {
        AtomMetadata {
            source: atom::SourceType::UserInput,
            agent_id: Uuid::nil(),
            conversation_id: None,
            tags: vec![],
            importance: 1.0,
        }
    }

    #[tokio::test]
    async fn test_engine_ingest_and_query() {
        let engine = InfiniteContextEngine::new().await.unwrap();
        let ids = engine
            .ingest("hello world test", test_metadata())
            .await
            .unwrap();
        assert_eq!(ids.len(), 1);

        let results = engine.query("hello world", 5).await.unwrap();
        assert!(!results.atoms.is_empty());
    }

    #[tokio::test]
    async fn test_engine_assemble_context() {
        let engine = InfiniteContextEngine::new().await.unwrap();
        engine
            .ingest(
                "The quick brown fox jumps over the lazy dog",
                test_metadata(),
            )
            .await
            .unwrap();

        let context = engine.assemble_context("fox", 1000).await.unwrap();
        assert!(context.contains("fox"));
    }

    #[tokio::test]
    async fn test_engine_atom_count() {
        let engine = InfiniteContextEngine::new().await.unwrap();
        assert_eq!(engine.atom_count().await.unwrap(), 0);
        engine.ingest("test", test_metadata()).await.unwrap();
        assert_eq!(engine.atom_count().await.unwrap(), 1);
    }
}
