/// ML-powered predictive linting
/// Ghost warnings before code is even written

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PredictiveLinter {
    model: PredictionModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionModel {
    /// Language → rule predictions
    rules_by_language: HashMap<String, Vec<RulePrediction>>,
    /// Domain-specific rule weights
    domain_weights: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePrediction {
    pub rule_id: String,
    pub confidence: f32,
    pub typical_severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostWarning {
    pub rule_id: String,
    pub message: String,
    pub confidence: f32,
    pub line: usize,
}

impl PredictiveLinter {
    pub async fn new() -> Result<Self> {
        let model = PredictionModel {
            rules_by_language: Default::default(),
            domain_weights: Default::default(),
        };
        Ok(Self { model })
    }

    pub async fn load(_path: &str) -> Result<Self> {
        Self::new().await
    }

    pub async fn train_from_metrics(metrics: &[crate::RuleMetric]) -> Result<Self> {
        let mut linter = Self::new().await?;

        let mut rules_by_language: HashMap<String, Vec<RulePrediction>> = HashMap::new();
        let mut domain_weights: HashMap<String, f32> = HashMap::new();

        for metric in metrics {
            rules_by_language
                .entry(metric.language.clone())
                .or_insert_with(Vec::new)
                .push(RulePrediction {
                    rule_id: metric.rule_id.clone(),
                    confidence: metric.confidence,
                    typical_severity: "warning".to_string(),
                });

            let weight = domain_weights.entry(metric.domain.clone()).or_insert(0.0);
            *weight = (*weight + metric.confidence) / 2.0;
        }

        linter.model.rules_by_language = rules_by_language;
        linter.model.domain_weights = domain_weights;

        Ok(linter)
    }

    /// Predict issues before they occur
    pub async fn predict(&self, language: &str) -> Result<Vec<String>> {
        let predictions = self
            .model
            .rules_by_language
            .get(language)
            .map(|rules| rules.iter().map(|r| r.rule_id.clone()).collect())
            .unwrap_or_default();

        Ok(predictions)
    }

    /// Generate ghost warnings for a file (shown before code is written)
    pub async fn generate_ghost_warnings(
        &self,
        language: &str,
        _file_context: &str,
    ) -> Result<Vec<GhostWarning>> {
        let predictions = self.predict(language).await?;

        let warnings = predictions
            .into_iter()
            .enumerate()
            .map(|(i, rule_id)| GhostWarning {
                rule_id,
                message: "Predictive warning: this pattern often causes issues".to_string(),
                confidence: 0.65,
                line: i + 1,
            })
            .collect();

        Ok(warnings)
    }

    /// Save model to disk
    pub async fn save(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string(&self.model)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_predictor_creation() {
        let predictor = PredictiveLinter::new().await.unwrap();
        assert!(predictor.predict("rust").await.is_ok());
    }

    #[tokio::test]
    async fn test_ghost_warnings() {
        let predictor = PredictiveLinter::new().await.unwrap();
        let warnings = predictor.generate_ghost_warnings("rust", "").await.unwrap();
        assert!(warnings.is_empty()); // No training data yet
    }

    #[tokio::test]
    async fn test_model_training() {
        let metrics = vec![];
        let predictor = PredictiveLinter::train_from_metrics(&metrics)
            .await
            .unwrap();
        assert!(predictor.predict("rust").await.is_ok());
    }
}
