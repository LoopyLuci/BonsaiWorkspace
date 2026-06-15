//! BM25 Relevance Ranking Algorithm
//!
//! Probabilistic relevance framework with term frequency and inverse document frequency.
//! Industry-standard for full-text search ranking.

use std::collections::HashMap;

const K1: f32 = 1.5; // Term frequency saturation parameter
const B: f32 = 0.75; // Length normalization parameter

/// BM25 scorer
pub struct BM25 {
    // Document term frequencies: doc_id -> term -> frequency
    term_freqs: HashMap<u32, HashMap<String, u32>>,
    // Inverse document frequencies: term -> count
    doc_freqs: HashMap<String, u32>,
    // Average document length
    avg_doc_len: f32,
    // Total documents indexed
    total_docs: u32,
    // Document lengths: doc_id -> length
    doc_lengths: HashMap<u32, u32>,
}

impl BM25 {
    pub fn new() -> Self {
        Self {
            term_freqs: HashMap::new(),
            doc_freqs: HashMap::new(),
            avg_doc_len: 0.0,
            total_docs: 0,
            doc_lengths: HashMap::new(),
        }
    }

    /// Add a document to the index
    pub fn add_document(&mut self, doc_id: u32, terms: Vec<String>) {
        let doc_len = terms.len() as u32;
        self.doc_lengths.insert(doc_id, doc_len);

        let mut term_map = HashMap::new();
        for term in terms {
            *term_map.entry(term.clone()).or_insert(0) += 1;
            *self.doc_freqs.entry(term).or_insert(0) += 1;
        }

        self.term_freqs.insert(doc_id, term_map);
        self.total_docs += 1;

        // Recalculate average document length
        let total_len: u32 = self.doc_lengths.values().sum();
        self.avg_doc_len = total_len as f32 / self.total_docs as f32;
    }

    /// Score a query against indexed documents
    pub fn score_query(&self, query_terms: &[String]) -> Vec<(u32, f32)> {
        let mut scores: HashMap<u32, f32> = HashMap::new();

        for query_term in query_terms {
            let idf = self.idf(query_term);

            // Iterate over documents containing this term
            for (doc_id, term_freqs) in &self.term_freqs {
                if let Some(&tf) = term_freqs.get(query_term) {
                    let doc_len = self.doc_lengths[doc_id];
                    let score = self.bm25_score(tf as f32, idf, doc_len);
                    *scores.entry(*doc_id).or_insert(0.0) += score;
                }
            }
        }

        let mut results: Vec<(u32, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    /// Calculate IDF (Inverse Document Frequency)
    fn idf(&self, term: &str) -> f32 {
        let doc_count = self.doc_freqs.get(term).copied().unwrap_or(0) as f32;
        if doc_count == 0.0 {
            0.0
        } else {
            ((self.total_docs as f32 - doc_count + 0.5) / (doc_count + 0.5) + 1.0).ln()
        }
    }

    /// Calculate BM25 score for a term in a document
    fn bm25_score(&self, tf: f32, idf: f32, doc_len: u32) -> f32 {
        let doc_len = doc_len as f32;
        let numerator = tf * (K1 + 1.0);
        let denominator = tf + K1 * (1.0 - B + B * (doc_len / self.avg_doc_len));
        idf * (numerator / denominator)
    }

    /// Get statistics
    pub fn stats(&self) -> BM25Stats {
        BM25Stats {
            total_docs: self.total_docs,
            unique_terms: self.doc_freqs.len(),
            avg_doc_length: self.avg_doc_len,
        }
    }
}

pub struct BM25Stats {
    pub total_docs: u32,
    pub unique_terms: usize,
    pub avg_doc_length: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_basic() {
        let mut bm25 = BM25::new();

        bm25.add_document(1, vec!["hello".to_string(), "world".to_string()]);
        bm25.add_document(2, vec!["hello".to_string(), "there".to_string()]);
        bm25.add_document(3, vec!["goodbye".to_string(), "world".to_string()]);

        let results = bm25.score_query(&["hello".to_string()]);
        assert!(!results.is_empty());
        // Documents 1 and 2 contain "hello"
        assert!(results.iter().any(|(doc_id, _)| *doc_id == 1));
        assert!(results.iter().any(|(doc_id, _)| *doc_id == 2));
    }

    #[test]
    fn test_bm25_ranking() {
        let mut bm25 = BM25::new();

        // Doc 1: "hello" appears 5 times
        bm25.add_document(1, vec![
            "hello".to_string(), "hello".to_string(), "hello".to_string(),
            "hello".to_string(), "hello".to_string(), "filler".to_string(),
        ]);

        // Doc 2: "hello" appears once
        bm25.add_document(2, vec!["hello".to_string(), "other".to_string()]);

        let results = bm25.score_query(&["hello".to_string()]);
        // Doc 1 should rank higher due to higher frequency
        assert_eq!(results[0].0, 1);
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn test_bm25_idf() {
        let mut bm25 = BM25::new();

        // Common term in all documents
        bm25.add_document(1, vec!["the".to_string()]);
        bm25.add_document(2, vec!["the".to_string()]);
        bm25.add_document(3, vec!["the".to_string()]);

        // Rare term in one document
        bm25.add_document(4, vec!["xenophobic".to_string()]);

        let results = bm25.score_query(&["the".to_string(), "xenophobic".to_string()]);
        // Document 4 should rank higher (rare term boost)
        assert_eq!(results[0].0, 4);
    }

    #[test]
    fn test_bm25_stats() {
        let mut bm25 = BM25::new();
        bm25.add_document(1, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        bm25.add_document(2, vec!["d".to_string(), "e".to_string()]);

        let stats = bm25.stats();
        assert_eq!(stats.total_docs, 2);
        assert_eq!(stats.unique_terms, 5);
    }
}
