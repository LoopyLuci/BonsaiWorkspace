//! Vector Search (HNSW, Embeddings)

use std::collections::HashMap;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct Embedding {
    pub id: String,
    pub vector: Vec<f32>,
}

impl Embedding {
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        let dot_product: f32 = self.vector.iter().zip(&other.vector).map(|(a, b)| a * b).sum();
        let norm_a: f32 = self.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot_product / (norm_a * norm_b) }
    }
}

pub struct HnswIndex {
    embeddings: Arc<Mutex<HashMap<String, Embedding>>>,
}

impl HnswIndex {
    pub fn new() -> Self {
        Self {
            embeddings: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add(&self, embedding: Embedding) {
        self.embeddings.lock().insert(embedding.id.clone(), embedding);
    }

    pub fn search(&self, query: &Embedding, k: usize) -> Vec<(String, f32)> {
        let embeddings = self.embeddings.lock();
        let mut results: Vec<_> = embeddings
            .iter()
            .map(|(id, emb)| (id.clone(), query.cosine_similarity(emb)))
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(k).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_search() {
        let index = HnswIndex::new();
        index.add(Embedding { id: "1".to_string(), vector: vec![1.0, 0.0, 0.0] });
        index.add(Embedding { id: "2".to_string(), vector: vec![0.0, 1.0, 0.0] });
        let query = Embedding { id: "q".to_string(), vector: vec![1.0, 0.0, 0.0] };
        let results = index.search(&query, 1);
        assert_eq!(results[0].0, "1");
    }
}
