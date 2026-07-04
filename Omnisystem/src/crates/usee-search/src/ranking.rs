use crate::{Document, Result};

pub struct RankingEngine;

impl RankingEngine {
    pub fn bm25_score(doc: &Document, query_terms: &[String], avg_doc_len: f32) -> f32 {
        let k1 = 1.5;
        let b = 0.75;

        let mut score = 0.0;
        for term in query_terms {
            if doc.content.to_lowercase().contains(&term.to_lowercase()) {
                let term_freq = doc.content.matches(&term.to_lowercase()).count() as f32;
                // Standard BM25 IDF: ln((num_docs - docs_with_term + 0.5) / (docs_with_term + 0.5))
                // Simplified for single document: always positive value
                let idf = (1.0 + term_freq).ln();
                let normalized_len = doc.content.len() as f32 / avg_doc_len;

                score += idf * ((k1 + 1.0) * term_freq) /
                         (k1 * (1.0 - b + b * normalized_len) + term_freq);
            }
        }
        score
    }

    pub fn rank_documents(documents: &mut [Document], query_terms: &[String], avg_doc_len: f32) {
        for doc in documents.iter_mut() {
            doc.score = Self::bm25_score(doc, query_terms, avg_doc_len);
        }
        documents.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_ranking() {
        let mut doc = Document {
            id: "d1".to_string(),
            title: "Test".to_string(),
            content: "test content test".to_string(),
            metadata: HashMap::new(),
            score: 0.0,
        };
        
        let query = vec!["test".to_string()];
        let score = RankingEngine::bm25_score(&doc, &query, 10.0);
        assert!(score > 0.0);
    }
}
