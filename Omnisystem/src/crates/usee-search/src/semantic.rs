use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Vector embedding (semantic representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub dimension: usize,
    pub model: String,
}

impl Embedding {
    pub fn new(vector: Vec<f32>, model: String) -> Self {
        let dimension = vector.len();
        Embedding { vector, dimension, model }
    }

    /// Cosine similarity between two embeddings
    pub fn similarity(&self, other: &Embedding) -> f32 {
        let mut dot_product = 0.0;
        let mut mag_a = 0.0;
        let mut mag_b = 0.0;

        for (a, b) in self.vector.iter().zip(other.vector.iter()) {
            dot_product += a * b;
            mag_a += a * a;
            mag_b += b * b;
        }

        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        dot_product / (mag_a.sqrt() * mag_b.sqrt())
    }
}

/// Semantic search index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticIndex {
    pub embeddings: Vec<(String, Embedding)>, // (doc_id, embedding)
    pub model: String,
}

impl SemanticIndex {
    pub fn new(model: String) -> Self {
        SemanticIndex {
            embeddings: vec![],
            model,
        }
    }

    pub fn add_document(&mut self, doc_id: String, embedding: Embedding) {
        self.embeddings.push((doc_id, embedding));
    }

    /// Find top-k most similar documents
    pub fn search(&self, query_embedding: &Embedding, top_k: usize) -> Vec<(String, f32)> {
        let mut results: Vec<(String, f32)> = self
            .embeddings
            .iter()
            .map(|(doc_id, emb)| (doc_id.clone(), query_embedding.similarity(emb)))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(top_k).collect()
    }
}

/// Semantic analyzer for text embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalyzer {
    pub model: String,
    pub dimension: usize,
    pub vocab_size: usize,
}

impl SemanticAnalyzer {
    pub fn new(model: String, dimension: usize) -> Self {
        SemanticAnalyzer {
            model,
            dimension,
            vocab_size: 50000,
        }
    }

    /// Mock embedding generation (in production: use actual ML model)
    pub fn embed_text(&self, text: &str) -> Result<Embedding> {
        let mut vector = vec![0.0; self.dimension];

        // Simple hash-based mock embedding for demonstration
        let hash = text.bytes().fold(0u64, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(b as u64)
        });

        for i in 0..self.dimension {
            let seed = hash.wrapping_add(i as u64);
            vector[i] = ((seed as f32).sin() + 1.0) / 2.0;
        }

        Ok(Embedding::new(vector, self.model.clone()))
    }

    /// Normalize embedding
    pub fn normalize(&self, embedding: &Embedding) -> Embedding {
        let magnitude: f32 = embedding.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude == 0.0 {
            return embedding.clone();
        }

        let normalized = embedding.vector.iter().map(|x| x / magnitude).collect();
        Embedding::new(normalized, embedding.model.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_creation() {
        let vec = vec![0.1, 0.2, 0.3, 0.4];
        let emb = Embedding::new(vec, "test-model".to_string());
        assert_eq!(emb.dimension, 4);
    }

    #[test]
    fn test_cosine_similarity() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];
        let emb1 = Embedding::new(vec1, "model".to_string());
        let emb2 = Embedding::new(vec2, "model".to_string());
        let sim = emb1.similarity(&emb2);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_semantic_index() {
        let mut index = SemanticIndex::new("test".to_string());
        let emb1 = Embedding::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let emb2 = Embedding::new(vec![0.9, 0.1, 0.0], "test".to_string());

        index.add_document("doc1".to_string(), emb1);
        index.add_document("doc2".to_string(), emb2);

        let query = Embedding::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let results = index.search(&query, 2);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_semantic_analyzer() {
        let analyzer = SemanticAnalyzer::new("test-model".to_string(), 128);
        let emb = analyzer.embed_text("hello world").unwrap();
        assert_eq!(emb.dimension, 128);
    }

    #[test]
    fn test_normalization() {
        let analyzer = SemanticAnalyzer::new("test".to_string(), 3);
        let emb = Embedding::new(vec![3.0, 4.0, 0.0], "test".to_string());
        let normalized = analyzer.normalize(&emb);
        let magnitude: f32 = normalized.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
    }
}
