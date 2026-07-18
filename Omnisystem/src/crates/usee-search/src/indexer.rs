use crate::{Document, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct Indexer {
    term_frequency: Arc<DashMap<String, std::collections::HashMap<String, f32>>>,
    doc_frequency: Arc<DashMap<String, u64>>,
}

impl Indexer {
    pub fn new() -> Self {
        Self {
            term_frequency: Arc::new(DashMap::new()),
            doc_frequency: Arc::new(DashMap::new()),
        }
    }

    pub fn build_index(&self, document: &Document) -> Result<()> {
        let tokens = self.tokenize(&document.content);
        let mut tf_map = std::collections::HashMap::new();
        
        for token in tokens {
            *tf_map.entry(token.clone()).or_insert(0.0) += 1.0;
            if let Some(mut entry) = self.doc_frequency.get_mut(&token) {
                *entry += 1;
            } else {
                self.doc_frequency.insert(token, 1);
            }
        }
        
        self.term_frequency.insert(document.id.clone(), tf_map);
        Ok(())
    }

    pub fn get_term_frequency(&self, doc_id: &str) -> Option<std::collections::HashMap<String, f32>> {
        self.term_frequency.get(doc_id).map(|ref_| ref_.value().clone())
    }

    pub fn get_doc_frequency(&self, term: &str) -> u64 {
        self.doc_frequency.get(term).map(|ref_| *ref_.value()).unwrap_or(0)
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .filter(|s| s.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_indexer_creation() {
        let indexer = Indexer::new();
        assert_eq!(indexer.get_doc_frequency("test"), 0);
    }

    #[test]
    fn test_build_index() {
        let indexer = Indexer::new();
        let doc = Document {
            id: "doc1".to_string(),
            title: "Test".to_string(),
            content: "test content test".to_string(),
            metadata: HashMap::new(),
            score: 1.0,
        };
        assert!(indexer.build_index(&doc).is_ok());
    }
}
