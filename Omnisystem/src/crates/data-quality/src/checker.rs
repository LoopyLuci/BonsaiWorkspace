use crate::{QualityRule, DataProfile, Anomaly, ValidationResult, QualityError, QualityResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct QualityChecker {
    rules: Arc<DashMap<Uuid, QualityRule>>,
    profiles: Arc<DashMap<Uuid, DataProfile>>,
    anomalies: Arc<DashMap<Uuid, Anomaly>>,
    results: Arc<DashMap<Uuid, ValidationResult>>,
}

impl QualityChecker {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
            profiles: Arc::new(DashMap::new()),
            anomalies: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_rule(&self, rule: &QualityRule) -> QualityResult<()> {
        self.rules.insert(rule.rule_id, rule.clone());
        Ok(())
    }

    pub async fn profile_data(&self, profile: &DataProfile) -> QualityResult<()> {
        self.profiles.insert(profile.profile_id, profile.clone());
        Ok(())
    }

    pub async fn detect_anomaly(&self, anomaly: &Anomaly) -> QualityResult<()> {
        self.anomalies.insert(anomaly.anomaly_id, anomaly.clone());
        Ok(())
    }

    pub async fn validate_data(&self, result: &ValidationResult) -> QualityResult<()> {
        self.results.insert(result.result_id, result.clone());
        Ok(())
    }

    pub async fn check_quality(&self, dataset_id: Uuid) -> QualityResult<ValidationResult> {
        let rules: Vec<_> = self.rules.iter().filter(|r| r.value().dataset_id == dataset_id).map(|r| r.value().clone()).collect();

        let passed = !rules.is_empty();

        Ok(ValidationResult {
            result_id: Uuid::new_v4(),
            dataset_id,
            passed,
            violations: vec![],
        })
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for QualityChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_rule() {
        let checker = QualityChecker::new();
        let rule = QualityRule {
            rule_id: Uuid::new_v4(),
            name: "null_check".to_string(),
            dataset_id: Uuid::new_v4(),
            threshold: 0.95,
        };

        checker.add_rule(&rule).await.unwrap();
        assert_eq!(checker.rule_count(), 1);
    }

    #[tokio::test]
    async fn test_profile_data() {
        let checker = QualityChecker::new();
        let profile = DataProfile {
            profile_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            null_count: 100,
            unique_count: 1000,
            min_value: 0.0,
            max_value: 100.0,
        };

        checker.profile_data(&profile).await.unwrap();
    }

    #[tokio::test]
    async fn test_detect_anomaly() {
        let checker = QualityChecker::new();
        let anomaly = Anomaly {
            anomaly_id: Uuid::new_v4(),
            dataset_id: Uuid::new_v4(),
            record_index: 5000,
            anomaly_score: 0.95,
        };

        checker.detect_anomaly(&anomaly).await.unwrap();
    }

    #[tokio::test]
    async fn test_check_quality() {
        let checker = QualityChecker::new();
        let dataset_id = Uuid::new_v4();
        let rule = QualityRule {
            rule_id: Uuid::new_v4(),
            name: "completeness".to_string(),
            dataset_id,
            threshold: 0.98,
        };

        checker.add_rule(&rule).await.unwrap();
        let result = checker.check_quality(dataset_id).await.unwrap();
        assert!(result.passed);
    }
}
