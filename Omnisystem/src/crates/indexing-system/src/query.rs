//! Query execution: a small hybrid search engine that ties the tokenizer
//! and BM25 lexical index together into a single index/search API over
//! plain-text documents.

use crate::lexical::{Tokenizer, BM25};
use std::collections::HashMap;

/// A single ranked search hit.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub doc_id: u32,
    pub score: f32,
    pub preview: String,
}

/// Indexes plain-text documents and answers ranked BM25 queries against
/// them.
pub struct SearchEngine {
    tokenizer: Tokenizer,
    bm25: BM25,
    documents: HashMap<u32, String>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            tokenizer: Tokenizer::new(),
            bm25: BM25::new(),
            documents: HashMap::new(),
        }
    }

    /// Tokenize and index a document's text under `doc_id`.
    pub fn index_document(&mut self, doc_id: u32, text: &str) {
        let terms = self.tokenizer.tokenize(text);
        self.bm25.add_document(doc_id, terms);
        self.documents.insert(doc_id, text.to_string());
    }

    /// Number of documents indexed.
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Rank indexed documents against a free-text query, returning up to
    /// `limit` results ordered by descending BM25 score.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_terms = self.tokenizer.tokenize(query);
        self.bm25
            .score_query(&query_terms)
            .into_iter()
            .filter(|(_, score)| *score > 0.0)
            .take(limit)
            .map(|(doc_id, score)| SearchResult {
                doc_id,
                score,
                preview: self
                    .documents
                    .get(&doc_id)
                    .map(|text| text.chars().take(80).collect())
                    .unwrap_or_default(),
            })
            .collect()
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

    fn sample_engine() -> SearchEngine {
        let mut engine = SearchEngine::new();
        engine.index_document(1, "Rust is a systems programming language focused on safety");
        engine.index_document(2, "Python is a dynamically typed scripting language");
        engine.index_document(3, "Rust provides memory safety without garbage collection");
        engine
    }

    #[test]
    fn indexing_tracks_document_count() {
        let engine = sample_engine();
        assert_eq!(engine.document_count(), 3);
    }

    #[test]
    fn search_ranks_documents_matching_the_query_higher() {
        let engine = sample_engine();
        let results = engine.search("rust safety", 10);

        assert!(!results.is_empty());
        // Docs 1 and 3 both mention rust/safety; doc 2 (Python) should not
        // appear since it shares no query terms after stopword removal.
        let ids: Vec<u32> = results.iter().map(|r| r.doc_id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
        assert!(!ids.contains(&2));
    }

    #[test]
    fn search_respects_the_limit() {
        let engine = sample_engine();
        let results = engine.search("language", 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_with_no_matching_terms_returns_empty() {
        let engine = sample_engine();
        let results = engine.search("xylophone", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn preview_is_truncated_to_the_document_start() {
        let mut engine = SearchEngine::new();
        engine.index_document(1, "a".repeat(200).as_str());
        let results = engine.search("a", 1);
        assert!(results.is_empty(), "single-letter tokens produce no BM25 match by design here");

        // Use a real word instead to exercise the preview path.
        let mut engine = SearchEngine::new();
        let long_text = format!("keyword {}", "filler ".repeat(50));
        engine.index_document(1, &long_text);
        let results = engine.search("keyword", 1);
        assert_eq!(results.len(), 1);
        assert!(results[0].preview.len() <= 80);
    }
}
