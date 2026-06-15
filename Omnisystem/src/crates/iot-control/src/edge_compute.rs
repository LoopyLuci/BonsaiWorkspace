use crate::{Result, IotError};
use dashmap::DashMap;
use std::sync::Arc;

pub struct EdgeCompute {
    rules: Arc<DashMap<String, Rule>>,
    cache: Arc<DashMap<String, Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub action: String,
    pub enabled: bool,
}

impl EdgeCompute {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(DashMap::new()),
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn add_rule(&self, rule: Rule) -> Result<()> {
        self.rules.insert(rule.id.clone(), rule);
        tracing::info!("Edge rule added");
        Ok(())
    }

    pub fn execute_rule(&self, rule_id: &str, input: &[u8]) -> Result<Vec<u8>> {
        if let Some(rule) = self.rules.get(rule_id) {
            if !rule.enabled {
                return Err(IotError::ProtocolError("Rule disabled".to_string()));
            }
            tracing::info!("Executing rule: {}", rule.name);
            Ok(input.to_vec())
        } else {
            Err(IotError::ProtocolError(format!("Rule not found: {}", rule_id)))
        }
    }

    pub fn cache_data(&self, key: String, data: Vec<u8>) -> Result<()> {
        self.cache.insert(key, data);
        Ok(())
    }

    pub fn get_cached(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.get(key).map(|ref_| ref_.value().clone())
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for EdgeCompute {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_compute_creation() {
        let ec = EdgeCompute::new();
        assert_eq!(ec.rule_count(), 0);
    }

    #[test]
    fn test_add_rule() {
        let ec = EdgeCompute::new();
        let rule = Rule {
            id: "r1".to_string(),
            name: "Test Rule".to_string(),
            condition: "temp > 25".to_string(),
            action: "alert".to_string(),
            enabled: true,
        };
        assert!(ec.add_rule(rule).is_ok());
        assert_eq!(ec.rule_count(), 1);
    }

    #[test]
    fn test_execute_rule() {
        let ec = EdgeCompute::new();
        let rule = Rule {
            id: "r1".to_string(),
            name: "Test Rule".to_string(),
            condition: "temp > 25".to_string(),
            action: "alert".to_string(),
            enabled: true,
        };
        ec.add_rule(rule).unwrap();
        let result = ec.execute_rule("r1", &[1, 2, 3]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_data() {
        let ec = EdgeCompute::new();
        let data = vec![1, 2, 3, 4, 5];
        assert!(ec.cache_data("key1".to_string(), data.clone()).is_ok());
        assert_eq!(ec.get_cached("key1"), Some(data));
    }
}
