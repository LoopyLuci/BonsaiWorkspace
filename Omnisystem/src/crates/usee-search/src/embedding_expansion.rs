use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ExpandedEmbedding {
    pub doc_id: String,
    pub base_embedding: Vec<f32>,
    pub contextual_features: Vec<f32>,
    pub semantic_neighbors: Vec<String>,
}

pub struct EmbeddingExpander {
    embeddings: Arc<DashMap<String, ExpandedEmbedding>>,
}

impl EmbeddingExpander {
    pub fn new() -> Self {
        Self {
            embeddings: Arc::new(DashMap::new()),
        }
    }

    pub fn add_embedding(&self, doc_id: String, base_embedding: Vec<f32>, contextual_features: Vec<f32>) {
        let expanded = ExpandedEmbedding {
            doc_id,
            base_embedding,
            contextual_features,
            semantic_neighbors: Vec::new(),
        };
        self.embeddings.insert(expanded.doc_id.clone(), expanded);
    }

    pub fn add_neighbor(&self, doc_id: &str, neighbor_id: String) -> bool {
        if let Some(mut embedding) = self.embeddings.get_mut(doc_id) {
            embedding.semantic_neighbors.push(neighbor_id);
            true
        } else {
            false
        }
    }

    pub fn get_expanded_embedding(&self, doc_id: &str) -> Option<ExpandedEmbedding> {
        self.embeddings.get(doc_id).map(|e| e.clone())
    }

    pub fn expand_with_context(&self, doc_id: &str, context_vector: &[f32]) -> Option<Vec<f32>> {
        if let Some(embedding) = self.embeddings.get(doc_id) {
            let mut expanded = embedding.base_embedding.clone();
            for (i, &ctx) in context_vector.iter().enumerate() {
                if i < expanded.len() {
                    expanded[i] = (expanded[i] + ctx) / 2.0;
                }
            }
            Some(expanded)
        } else {
            None
        }
    }

    pub fn embedding_count(&self) -> usize {
        self.embeddings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_expansion() {
        let ee = EmbeddingExpander::new();
        ee.add_embedding(
            "doc1".to_string(),
            vec![0.1, 0.2, 0.3],
            vec![0.5, 0.6],
        );
        assert_eq!(ee.embedding_count(), 1);
    }

    #[test]
    fn test_neighbor_addition() {
        let ee = EmbeddingExpander::new();
        ee.add_embedding(
            "doc1".to_string(),
            vec![0.1, 0.2, 0.3],
            vec![0.5, 0.6],
        );
        assert!(ee.add_neighbor("doc1", "doc2".to_string()));
        let emb = ee.get_expanded_embedding("doc1").unwrap();
        assert_eq!(emb.semantic_neighbors.len(), 1);
    }

    #[test]
    fn test_context_expansion() {
        let ee = EmbeddingExpander::new();
        ee.add_embedding(
            "doc1".to_string(),
            vec![0.1, 0.2, 0.3],
            vec![0.5, 0.6],
        );
        let context = vec![0.5, 0.4, 0.2];
        let expanded = ee.expand_with_context("doc1", &context).unwrap();
        assert_eq!(expanded.len(), 3);
    }
}
