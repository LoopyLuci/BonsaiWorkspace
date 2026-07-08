use crate::{Embedding, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct EmbeddingStore {
    embeddings: Arc<DashMap<String, Embedding>>,
}

impl EmbeddingStore {
    pub fn new() -> Self {
        Self {
            embeddings: Arc::new(DashMap::new()),
        }
    }

    pub fn store_embedding(&self, embedding: Embedding) -> Result<()> {
        self.embeddings.insert(embedding.document_id.clone(), embedding);
        Ok(())
    }

    pub fn get_embedding(&self, doc_id: &str) -> Option<Embedding> {
        self.embeddings.get(doc_id).map(|ref_| ref_.value().clone())
    }

    pub fn similarity(&self, vec1: &[f32], vec2: &[f32]) -> f32 {
        vec1.iter().zip(vec2).map(|(a, b)| a * b).sum()
    }

    pub fn embedding_count(&self) -> usize {
        self.embeddings.len()
    }
}

impl Default for EmbeddingStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_store() {
        let store = EmbeddingStore::new();
        let emb = Embedding {
            document_id: "doc1".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            dimensions: 3,
        };
        assert!(store.store_embedding(emb).is_ok());
        assert_eq!(store.embedding_count(), 1);
    }

    #[test]
    fn test_similarity() {
        let store = EmbeddingStore::new();
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        let sim = store.similarity(&v1, &v2);
        assert_eq!(sim, 1.0);
    }
}
