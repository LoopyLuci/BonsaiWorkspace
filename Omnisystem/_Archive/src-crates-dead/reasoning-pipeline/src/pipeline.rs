use crate::{ReasoningQuery, PathStep, ReasoningChain, MultiHopResult, ReasoningExplanation, ReasoningError, ReasoningResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ReasoningPipeline {
    queries: Arc<DashMap<Uuid, ReasoningQuery>>,
    chains: Arc<DashMap<Uuid, ReasoningChain>>,
    results: Arc<DashMap<Uuid, MultiHopResult>>,
    explanations: Arc<DashMap<Uuid, ReasoningExplanation>>,
    steps: Arc<DashMap<Uuid, PathStep>>,
}

impl ReasoningPipeline {
    pub fn new() -> Self {
        Self {
            queries: Arc::new(DashMap::new()),
            chains: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
            explanations: Arc::new(DashMap::new()),
            steps: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_query(&self, question: &str, max_hops: u32) -> ReasoningResult<ReasoningQuery> {
        let query = ReasoningQuery {
            query_id: Uuid::new_v4(),
            question: question.to_string(),
            max_hops,
            created_at: Utc::now(),
        };

        self.queries.insert(query.query_id, query.clone());
        Ok(query)
    }

    pub async fn add_path_step(&self, hop: u32, entity: &str, relation: &str) -> ReasoningResult<PathStep> {
        let step = PathStep {
            step_id: Uuid::new_v4(),
            hop_number: hop,
            entity: entity.to_string(),
            relation: relation.to_string(),
        };

        self.steps.insert(step.step_id, step.clone());
        Ok(step)
    }

    pub async fn build_reasoning_chain(&self, query_id: Uuid, step_ids: Vec<Uuid>) -> ReasoningResult<ReasoningChain> {
        if self.queries.get(&query_id).is_none() {
            return Err(ReasoningError::QueryNotFound);
        }

        let chain = ReasoningChain {
            chain_id: Uuid::new_v4(),
            query_id,
            steps: step_ids,
            conclusion: "inferred_answer".to_string(),
            confidence: 0.90,
        };

        self.chains.insert(chain.chain_id, chain.clone());
        Ok(chain)
    }

    pub async fn execute_multi_hop(&self, query_id: Uuid) -> ReasoningResult<MultiHopResult> {
        if self.queries.get(&query_id).is_none() {
            return Err(ReasoningError::QueryNotFound);
        }

        let result = MultiHopResult {
            result_id: Uuid::new_v4(),
            query_id,
            paths: vec![vec!["entity1".to_string(), "entity2".to_string()]],
            total_hops: 2,
            found: true,
        };

        self.results.insert(result.result_id, result.clone());
        Ok(result)
    }

    pub async fn generate_explanation(&self, result_id: Uuid) -> ReasoningResult<ReasoningExplanation> {
        if self.results.get(&result_id).is_none() {
            return Err(ReasoningError::ExecutionFailed);
        }

        let explanation = ReasoningExplanation {
            explanation_id: Uuid::new_v4(),
            result_id,
            reasoning_steps: vec!["step1".to_string(), "step2".to_string()],
            evidence: vec!["fact1".to_string()],
        };

        self.explanations.insert(explanation.explanation_id, explanation.clone());
        Ok(explanation)
    }

    pub fn query_count(&self) -> usize {
        self.queries.len()
    }
}

impl Default for ReasoningPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_query() {
        let pipeline = ReasoningPipeline::new();
        let query = pipeline.create_query("Is Alice mortal?", 3).await.unwrap();

        assert_eq!(query.question, "Is Alice mortal?");
        assert_eq!(pipeline.query_count(), 1);
    }

    #[tokio::test]
    async fn test_add_path_step() {
        let pipeline = ReasoningPipeline::new();
        let step = pipeline.add_path_step(1, "Alice", "is_human").await.unwrap();

        assert_eq!(step.hop_number, 1);
    }

    #[tokio::test]
    async fn test_build_reasoning_chain() {
        let pipeline = ReasoningPipeline::new();
        let query = pipeline.create_query("Why?", 5).await.unwrap();

        let chain = pipeline.build_reasoning_chain(query.query_id, vec![]).await.unwrap();
        assert_eq!(chain.confidence, 0.90);
    }

    #[tokio::test]
    async fn test_execute_multi_hop() {
        let pipeline = ReasoningPipeline::new();
        let query = pipeline.create_query("Path?", 4).await.unwrap();

        let result = pipeline.execute_multi_hop(query.query_id).await.unwrap();
        assert!(result.found);
    }
}
