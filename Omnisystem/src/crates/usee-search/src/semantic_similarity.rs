use std::collections::HashMap;

pub struct SemanticSimilarity {
    embeddings: HashMap<String, Vec<f32>>,
}

impl SemanticSimilarity {
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
        }
    }

    pub fn add_embedding(&mut self, doc_id: String, embedding: Vec<f32>) {
        self.embeddings.insert(doc_id, embedding);
    }

    pub fn cosine_similarity(vec_a: &[f32], vec_b: &[f32]) -> f32 {
        if vec_a.is_empty() || vec_b.is_empty() || vec_a.len() != vec_b.len() {
            return 0.0;
        }

        let dot_product: f32 = vec_a.iter().zip(vec_b).map(|(a, b)| a * b).sum();
        let mag_a: f32 = vec_a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = vec_b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            0.0
        } else {
            dot_product / (mag_a * mag_b)
        }
    }

    pub fn find_similar(&self, query_embedding: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut similarities: Vec<_> = self.embeddings
            .iter()
            .map(|(doc_id, embedding)| {
                let sim = Self::cosine_similarity(query_embedding, embedding);
                (doc_id.clone(), sim)
            })
            .collect();

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.into_iter().take(top_k).collect()
    }

    pub fn similarity_count(&self) -> usize {
        self.embeddings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        assert_eq!(SemanticSimilarity::cosine_similarity(&v1, &v2), 1.0);
    }

    #[test]
    fn test_semantic_embedding() {
        let mut ss = SemanticSimilarity::new();
        ss.add_embedding("doc1".to_string(), vec![1.0, 0.5, 0.3]);
        ss.add_embedding("doc2".to_string(), vec![0.9, 0.5, 0.4]);
        assert_eq!(ss.similarity_count(), 2);
    }

    #[test]
    fn test_find_similar() {
        let mut ss = SemanticSimilarity::new();
        ss.add_embedding("doc1".to_string(), vec![1.0, 0.0]);
        ss.add_embedding("doc2".to_string(), vec![0.0, 1.0]);
        let query = vec![1.0, 0.0];
        let results = ss.find_similar(&query, 2);
        assert_eq!(results.len(), 2);
    }
}
