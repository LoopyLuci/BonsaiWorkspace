use crate::{Rule, Fact, InferenceOutcome, KnowledgeBase, DeductiveStep, InferenceError, InferenceResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct InferenceEngine {
    rules: Arc<DashMap<Uuid, Rule>>,
    facts: Arc<DashMap<Uuid, Fact>>,
    inferences: Arc<DashMap<Uuid, InferenceOutcome>>,
    kb: Arc<DashMap<Uuid, KnowledgeBase>>,
    steps: Arc<DashMap<Uuid, DeductiveStep>>,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
            facts: Arc::new(DashMap::new()),
            inferences: Arc::new(DashMap::new()),
            kb: Arc::new(DashMap::new()),
            steps: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_rule(&self, name: &str, conditions: Vec<String>, conclusion: &str) -> InferenceResult<Rule> {
        let rule = Rule {
            rule_id: Uuid::new_v4(),
            name: name.to_string(),
            conditions,
            conclusion: conclusion.to_string(),
            priority: 0,
        };

        self.rules.insert(rule.rule_id, rule.clone());
        Ok(rule)
    }

    pub async fn add_fact(&self, predicate: &str, value: &str, confidence: f32) -> InferenceResult<Fact> {
        let fact = Fact {
            fact_id: Uuid::new_v4(),
            predicate: predicate.to_string(),
            value: value.to_string(),
            confidence,
        };

        self.facts.insert(fact.fact_id, fact.clone());
        Ok(fact)
    }

    pub async fn apply_rule(&self, rule_id: Uuid) -> InferenceResult<InferenceOutcome> {
        if self.rules.get(&rule_id).is_none() {
            return Err(InferenceError::RuleNotFound);
        }

        let inference = InferenceOutcome {
            result_id: Uuid::new_v4(),
            rule_id,
            derived_fact: "inferred_conclusion".to_string(),
            confidence: 0.85,
            inference_chain: vec![],
        };

        self.inferences.insert(inference.result_id, inference.clone());
        Ok(inference)
    }

    pub async fn query(&self, predicate: &str) -> InferenceResult<Vec<Fact>> {
        let mut results = Vec::new();

        for entry in self.facts.iter() {
            if entry.value().predicate == predicate {
                results.push(entry.value().clone());
            }
        }

        Ok(results)
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_rule() {
        let engine = InferenceEngine::new();
        let rule = engine
            .add_rule("mortal", vec!["is_human".to_string()], "is_mortal")
            .await
            .unwrap();

        assert_eq!(rule.name, "mortal");
        assert_eq!(engine.rule_count(), 1);
    }

    #[tokio::test]
    async fn test_add_fact() {
        let engine = InferenceEngine::new();
        let fact = engine.add_fact("species", "human", 1.0).await.unwrap();

        assert_eq!(fact.predicate, "species");
    }

    #[tokio::test]
    async fn test_apply_rule() {
        let engine = InferenceEngine::new();
        let rule = engine.add_rule("test", vec![], "result").await.unwrap();

        let inference = engine.apply_rule(rule.rule_id).await.unwrap();
        assert_eq!(inference.confidence, 0.85);
    }

    #[tokio::test]
    async fn test_query() {
        let engine = InferenceEngine::new();
        engine.add_fact("color", "red", 1.0).await.unwrap();
        engine.add_fact("color", "blue", 1.0).await.unwrap();

        let results = engine.query("color").await.unwrap();
        assert_eq!(results.len(), 2);
    }
}
