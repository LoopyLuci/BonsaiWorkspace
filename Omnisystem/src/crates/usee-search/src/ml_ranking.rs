use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RankingFeatures {
    pub tf_idf: f32,
    pub bm25: f32,
    pub pagerank: f32,
    pub freshness: f32,
    pub domain_authority: f32,
    pub user_engagement: f32,
    pub semantic_similarity: f32,
    pub query_position: f32,
    pub content_length: f32,
    pub ctr: f32,
}

pub struct MLRanker {
    model_weights: Arc<DashMap<usize, f32>>,
}

impl MLRanker {
    pub fn new() -> Self {
        let model_weights = Arc::new(DashMap::new());
        for i in 0..10 {
            model_weights.insert(i, 0.1);
        }
        Self { model_weights }
    }

    pub fn rank_score(&self, features: &RankingFeatures) -> f32 {
        let feature_vec = vec![
            features.tf_idf,
            features.bm25,
            features.pagerank,
            features.freshness,
            features.domain_authority,
            features.user_engagement,
            features.semantic_similarity,
            features.query_position,
            features.content_length,
            features.ctr,
        ];

        let mut score = 0.0;
        for (i, val) in feature_vec.iter().enumerate() {
            if let Some(weight) = self.model_weights.get(&i) {
                score += val * weight.value();
            }
        }
        
        (1.0 / (1.0 + (-score).exp())).clamp(0.0, 1.0)
    }

    pub fn update_weights(&self, feature_idx: usize, delta: f32) {
        if let Some(mut weight) = self.model_weights.get_mut(&feature_idx) {
            *weight += delta;
        }
    }

    pub fn get_weight(&self, feature_idx: usize) -> Option<f32> {
        self.model_weights.get(&feature_idx).map(|w| *w)
    }

    pub fn train_batch(&self, examples: Vec<(RankingFeatures, f32)>) {
        for (features, expected) in examples {
            let predicted = self.rank_score(&features);
            let error = expected - predicted;
            
            let feature_vec = vec![
                features.tf_idf,
                features.bm25,
                features.pagerank,
                features.freshness,
                features.domain_authority,
                features.user_engagement,
                features.semantic_similarity,
                features.query_position,
                features.content_length,
                features.ctr,
            ];

            for (i, val) in feature_vec.iter().enumerate() {
                let gradient = error * val * 0.01;
                self.update_weights(i, gradient);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranker_creation() {
        let ranker = MLRanker::new();
        assert_eq!(ranker.get_weight(0).unwrap(), 0.1);
    }

    #[test]
    fn test_ranking_score() {
        let ranker = MLRanker::new();
        let features = RankingFeatures {
            tf_idf: 0.8,
            bm25: 0.9,
            pagerank: 0.7,
            freshness: 0.6,
            domain_authority: 0.8,
            user_engagement: 0.5,
            semantic_similarity: 0.9,
            query_position: 0.4,
            content_length: 0.6,
            ctr: 0.7,
        };
        let score = ranker.rank_score(&features);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_training() {
        let ranker = MLRanker::new();
        let features = RankingFeatures {
            tf_idf: 0.8,
            bm25: 0.9,
            pagerank: 0.7,
            freshness: 0.6,
            domain_authority: 0.8,
            user_engagement: 0.5,
            semantic_similarity: 0.9,
            query_position: 0.4,
            content_length: 0.6,
            ctr: 0.7,
        };
        ranker.train_batch(vec![(features, 0.95)]);
        let new_weight = ranker.get_weight(0).unwrap();
        assert_ne!(new_weight, 0.1);
    }
}
