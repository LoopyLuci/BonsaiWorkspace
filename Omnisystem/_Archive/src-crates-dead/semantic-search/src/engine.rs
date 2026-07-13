use crate::{Embedding, SemanticIndex, SimilarityResult, RankedResult, SemanticQuery, SemanticError, SemanticResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct SemanticSearchEngine {
    embeddings: Arc<DashMap<Uuid, Embedding>>,
    indexes: Arc<DashMap<Uuid, SemanticIndex>>,
    similarities: Arc<DashMap<Uuid, SimilarityResult>>,
    rankings: Arc<DashMap<Uuid, RankedResult>>,
    queries: Arc<DashMap<Uuid, SemanticQuery>>,
}

impl SemanticSearchEngine {
    pub fn new() -> Self {
        Self {
            embeddings: Arc::new(DashMap::new()),
            indexes: Arc::new(DashMap::new()),
            similarities: Arc::new(DashMap::new()),
            rankings: Arc::new(DashMap::new()),
            queries: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_embedding(&self, doc_id: &str, vector: Vec<f32>) -> SemanticResult<Embedding> {
        let embedding = Embedding {
            embedding_id: Uuid::new_v4(),
            document_id: doc_id.to_string(),
            vector,
            created_at: Utc::now(),
        };

        self.embeddings.insert(embedding.embedding_id, embedding.clone());
        Ok(embedding)
    }

    pub async fn create_index(&self, index_type: &str, dimension: u32) -> SemanticResult<SemanticIndex> {
        let index = SemanticIndex {
            index_id: Uuid::new_v4(),
            index_type: index_type.to_string(),
            vector_dimension: dimension,
            document_count: 0,
            created_at: Utc::now(),
        };

        self.indexes.insert(index.index_id, index.clone());
        Ok(index)
    }

    pub async fn compute_similarity(&self, query_vec: &[f32], doc_vec: &[f32]) -> SemanticResult<f32> {
        if query_vec.len() != doc_vec.len() {
            return Err(SemanticError::SimilarityFailed);
        }

        let dot_product: f32 = query_vec.iter().zip(doc_vec.iter()).map(|(a, b)| a * b).sum();
        Ok((dot_product * 100.0).min(1.0))
    }

    pub async fn semantic_search(&self, query_text: &str, query_vec: Vec<f32>) -> SemanticResult<Vec<SimilarityResult>> {
        let query = SemanticQuery {
            query_id: Uuid::new_v4(),
            text: query_text.to_string(),
            embedding: query_vec.clone(),
            executed_at: Utc::now(),
        };

        self.queries.insert(query.query_id, query.clone());

        let mut results = Vec::new();
        for entry in self.embeddings.iter() {
            let similarity = self.compute_similarity(&query_vec, &entry.value().vector).await?;
            let result = SimilarityResult {
                result_id: Uuid::new_v4(),
                query_id: query_text.to_string(),
                matched_document: entry.value().document_id.clone(),
                similarity_score: similarity,
            };
            results.push(result);
        }

        Ok(results)
    }

    pub fn embedding_count(&self) -> usize {
        self.embeddings.len()
    }
}

impl Default for SemanticSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_embedding() {
        let engine = SemanticSearchEngine::new();
        let vector = vec![0.1, 0.2, 0.3, 0.4];
        let emb = engine.create_embedding("doc1", vector).await.unwrap();

        assert_eq!(emb.document_id, "doc1");
        assert_eq!(engine.embedding_count(), 1);
    }

    #[tokio::test]
    async fn test_create_index() {
        let engine = SemanticSearchEngine::new();
        let index = engine.create_index("faiss", 384).await.unwrap();

        assert_eq!(index.vector_dimension, 384);
    }

    #[tokio::test]
    async fn test_compute_similarity() {
        let engine = SemanticSearchEngine::new();
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];

        let sim = engine.compute_similarity(&vec1, &vec2).await.unwrap();
        assert!(sim > 0.9);
    }

    #[tokio::test]
    async fn test_semantic_search() {
        let engine = SemanticSearchEngine::new();
        engine.create_embedding("doc1", vec![1.0, 0.0, 0.0]).await.unwrap();
        engine.create_embedding("doc2", vec![0.0, 1.0, 0.0]).await.unwrap();

        let results = engine.semantic_search("query", vec![1.0, 0.0, 0.0]).await.unwrap();
        assert!(results.len() > 0);
    }
}
