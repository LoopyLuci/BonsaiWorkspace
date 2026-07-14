//! High-level facade tying together indexing, storage, and ranking.

use crate::indexer::Indexer;
use crate::ranking::RankingEngine;
use crate::{Document, Query, Result, SearchResult};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;

/// A single-node search engine: indexes documents and ranks them by BM25
/// relevance against a query.
pub struct SearchEngine {
    documents: Arc<DashMap<String, Document>>,
    indexer: Indexer,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(DashMap::new()),
            indexer: Indexer::new(),
        }
    }

    /// Index a document: builds term/doc-frequency stats and stores it for
    /// retrieval.
    pub fn index_document(&self, document: Document) -> Result<()> {
        self.indexer.build_index(&document)?;
        self.documents.insert(document.id.clone(), document);
        Ok(())
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Rank all indexed documents against the query using BM25 and return
    /// the top `query.limit` matches.
    pub fn search(&self, query: &Query) -> Result<SearchResult> {
        let start = Instant::now();

        let query_terms: Vec<String> = query
            .text
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let mut docs: Vec<Document> = self.documents.iter().map(|e| e.value().clone()).collect();

        let avg_doc_len = if docs.is_empty() {
            1.0
        } else {
            docs.iter().map(|d| d.content.len() as f32).sum::<f32>() / docs.len() as f32
        };

        RankingEngine::rank_documents(&mut docs, &query_terms, avg_doc_len);
        docs.retain(|d| d.score > 0.0);

        let limit = query.limit.max(1);
        docs.truncate(limit);

        let total = docs.len();
        Ok(SearchResult {
            documents: docs,
            total,
            query_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn doc(id: &str, content: &str) -> Document {
        Document {
            id: id.to_string(),
            title: id.to_string(),
            content: content.to_string(),
            metadata: HashMap::new(),
            score: 0.0,
        }
    }

    #[test]
    fn test_search_engine_new() {
        let engine = SearchEngine::new();
        assert_eq!(engine.document_count(), 0);
    }

    #[test]
    fn test_index_and_search() {
        let engine = SearchEngine::new();
        engine
            .index_document(doc("doc1", "rust programming language"))
            .unwrap();
        engine
            .index_document(doc("doc2", "python programming language"))
            .unwrap();
        assert_eq!(engine.document_count(), 2);

        let query = Query {
            text: "programming language".to_string(),
            limit: 10,
            offset: 0,
            filters: HashMap::new(),
        };
        let results = engine.search(&query).unwrap();
        assert!(results.total > 0);
    }

    #[test]
    fn test_search_respects_limit() {
        let engine = SearchEngine::new();
        for i in 0..5 {
            engine
                .index_document(doc(&format!("doc{}", i), "rust programming"))
                .unwrap();
        }
        let query = Query {
            text: "rust".to_string(),
            limit: 2,
            offset: 0,
            filters: HashMap::new(),
        };
        let results = engine.search(&query).unwrap();
        assert!(results.total <= 2);
    }
}
