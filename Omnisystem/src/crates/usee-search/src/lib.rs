//! usee-search: a full-text + vector search engine toolkit.
//!
//! [`core::SearchEngine`] is the single-node facade (index + BM25 rank).
//! Around it sit independent building blocks for distributed search:
//! caching, sharding/replication/federation, an inverted index, an ML
//! ranker, embeddings/semantic search, query parsing, and file
//! management. Several modules define same-named types for different
//! purposes (e.g. two independent `MLRanker`s in [`ml_ranking`] and
//! [`phase5_ml_ranking`], and two independent `Embedding`s in
//! [`semantic`] and [`types`]), so only the canonical [`types`] and
//! [`error`] items are glob-exported at the crate root; the rest are
//! reachable via their module paths.

pub mod ai_search;
pub mod caching;
pub mod connector;
pub mod core;
pub mod distributed;
pub mod embedding_expansion;
pub mod embeddings;
pub mod error;
pub mod federation;
pub mod file_management;
pub mod index_replication;
pub mod indexer;
pub mod ml_ranking;
pub mod phase4_indexing;
pub mod phase5_ml_ranking;
pub mod query;
pub mod ranking;
pub mod semantic;
pub mod semantic_similarity;
pub mod sharding;
pub mod types;

pub use core::SearchEngine;
pub use error::{Result, SearchError};
pub use types::*;
