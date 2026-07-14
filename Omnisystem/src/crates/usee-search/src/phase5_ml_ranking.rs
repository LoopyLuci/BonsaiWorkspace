use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankerModel {
    pub weights: [f32; 10],
    pub bias: f32,
}

#[derive(Clone, Debug)]
pub struct DocumentFeatures {
    pub tf_idf: f32,
    pub bm25: f32,
    pub pagerank: f32,
    pub freshness: f32,
    pub domain_authority: f32,
    pub user_engagement: f32,
    pub semantic_similarity: f32,
    pub query_match_position: f32,
    pub content_length: f32,
    pub click_through_rate: f32,
}

pub struct MLRanker {
    model: RankerModel,
    training_data: Vec<(DocumentFeatures, f32)>,
}

impl MLRanker {
    pub fn new() -> Self {
        MLRanker {
            model: RankerModel {
                weights: [0.2; 10],
                bias: 0.5,
            },
            training_data: Vec::new(),
        }
    }

    pub fn rank(&self, features: &DocumentFeatures) -> f32 {
        let feature_vec = [
            features.tf_idf,
            features.bm25,
            features.pagerank,
            features.freshness,
            features.domain_authority,
            features.user_engagement,
            features.semantic_similarity,
            features.query_match_position,
            features.content_length,
            features.click_through_rate,
        ];

        let score: f32 = self
            .model
            .weights
            .iter()
            .zip(feature_vec.iter())
            .map(|(w, f)| w * f)
            .sum::<f32>()
            + self.model.bias;

        1.0 / (1.0 + (-score).exp())
    }

    pub fn rank_documents(&self, mut docs: Vec<(String, DocumentFeatures)>) -> Vec<String> {
        docs.sort_by(|a, b| {
            let score_a = self.rank(&a.1);
            let score_b = self.rank(&b.1);
            score_b.partial_cmp(&score_a).unwrap()
        });
        docs.into_iter().map(|(id, _)| id).collect()
    }

    pub fn add_training_sample(&mut self, features: DocumentFeatures, relevance: f32) {
        self.training_data.push((features, relevance));
    }

    pub fn train_simple(&mut self) {
        if self.training_data.is_empty() {
            return;
        }

        for _ in 0..10 {
            for (features, target) in &self.training_data {
                let pred = self.rank(features);
                let error = target - pred;

                let feature_vec = [
                    features.tf_idf,
                    features.bm25,
                    features.pagerank,
                    features.freshness,
                    features.domain_authority,
                    features.user_engagement,
                    features.semantic_similarity,
                    features.query_match_position,
                    features.content_length,
                    features.click_through_rate,
                ];

                for i in 0..self.model.weights.len() {
                    self.model.weights[i] += 0.01 * error * feature_vec[i];
                }
                self.model.bias += 0.01 * error;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ml_ranker() {
        let ranker = MLRanker::new();
        let features = DocumentFeatures {
            tf_idf: 0.8,
            bm25: 0.7,
            pagerank: 0.9,
            freshness: 0.6,
            domain_authority: 0.85,
            user_engagement: 0.75,
            semantic_similarity: 0.88,
            query_match_position: 0.95,
            content_length: 0.5,
            click_through_rate: 0.7,
        };

        let score = ranker.rank(&features);
        assert!(score > 0.0 && score < 1.0);
    }

    #[test]
    fn test_ranking_documents() {
        let ranker = MLRanker::new();
        let docs = vec![
            (
                "doc1".to_string(),
                DocumentFeatures {
                    tf_idf: 0.5,
                    bm25: 0.5,
                    pagerank: 0.5,
                    freshness: 0.5,
                    domain_authority: 0.5,
                    user_engagement: 0.5,
                    semantic_similarity: 0.5,
                    query_match_position: 0.5,
                    content_length: 0.5,
                    click_through_rate: 0.5,
                },
            ),
            (
                "doc2".to_string(),
                DocumentFeatures {
                    tf_idf: 0.9,
                    bm25: 0.9,
                    pagerank: 0.9,
                    freshness: 0.9,
                    domain_authority: 0.9,
                    user_engagement: 0.9,
                    semantic_similarity: 0.9,
                    query_match_position: 0.9,
                    content_length: 0.9,
                    click_through_rate: 0.9,
                },
            ),
        ];

        let ranked = ranker.rank_documents(docs);
        assert_eq!(ranked[0], "doc2");
    }
}
