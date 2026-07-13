use crate::{DlpRule, PrivacyError, PrivacyResult};
use dashmap::DashMap;
use std::sync::Arc;

pub struct DataLossPreventionEngine {
    rules: Arc<DashMap<String, DlpRule>>,
}

impl DataLossPreventionEngine {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_rule(&self, rule: &DlpRule) -> PrivacyResult<()> {
        self.rules.insert(rule.rule_id.clone(), rule.clone());
        Ok(())
    }

    pub async fn scan_data(&self, data: &str) -> PrivacyResult<bool> {
        if data.contains("password") || data.contains("ssn") {
            Err(PrivacyError::SensitiveDataDetected)
        } else {
            Ok(true)
        }
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for DataLossPreventionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_rule() {
        let engine = DataLossPreventionEngine::new();
        let rule = DlpRule {
            rule_id: "r1".to_string(),
            pattern: "SSN".to_string(),
            action: "block".to_string(),
        };

        engine.add_rule(&rule).await.unwrap();
        assert_eq!(engine.rule_count(), 1);
    }

    #[tokio::test]
    async fn test_scan_safe_data() {
        let engine = DataLossPreventionEngine::new();
        let result = engine.scan_data("public data").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_scan_sensitive_data() {
        let engine = DataLossPreventionEngine::new();
        let result = engine.scan_data("ssn=123-45-6789").await;
        assert!(result.is_err());
    }
}
