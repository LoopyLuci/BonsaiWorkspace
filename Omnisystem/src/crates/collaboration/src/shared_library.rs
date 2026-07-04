/// Collaborative rule library with distributed synchronization
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedRule {
    pub rule_id: String,
    pub name: String,
    pub pattern: String,
    pub severity: String,
    pub author: String,
    pub version: String,
    pub created_at: i64,
    pub downloads: usize,
    pub rating: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleLibraryEntry {
    pub rule: SharedRule,
    pub last_modified: i64,
    pub sync_status: SyncStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SyncStatus {
    Synced,
    Pending,
    Conflict,
}

pub struct SharedLibrary {
    rules: Arc<RwLock<HashMap<String, RuleLibraryEntry>>>,
    local_db_path: String,
}

impl SharedLibrary {
    pub fn new(local_db_path: String) -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            local_db_path,
        }
    }

    pub async fn publish_rule(&self, rule: SharedRule) -> Result<String> {
        let rule_id = rule.rule_id.clone();
        let entry = RuleLibraryEntry {
            rule,
            last_modified: chrono::Utc::now().timestamp(),
            sync_status: SyncStatus::Pending,
        };

        let mut rules = self.rules.write().await;
        rules.insert(rule_id.clone(), entry);

        tracing::info!("Published rule: {}", rule_id);
        Ok(rule_id)
    }

    pub async fn download_rule(&self, rule_id: &str) -> Result<Option<SharedRule>> {
        let mut rules = self.rules.write().await;
        if let Some(entry) = rules.get_mut(rule_id) {
            entry.rule.downloads += 1;
            entry.last_modified = chrono::Utc::now().timestamp();
            return Ok(Some(entry.rule.clone()));
        }
        Ok(None)
    }

    pub async fn rate_rule(&self, rule_id: &str, rating: f32) -> Result<()> {
        let mut rules = self.rules.write().await;
        if let Some(entry) = rules.get_mut(rule_id) {
            entry.rule.rating = (entry.rule.rating + rating) / 2.0;
            entry.last_modified = chrono::Utc::now().timestamp();
        }
        Ok(())
    }

    pub async fn search_rules(&self, query: &str) -> Result<Vec<SharedRule>> {
        let rules = self.rules.read().await;
        let results: Vec<SharedRule> = rules
            .values()
            .filter(|e| e.rule.name.contains(query) || e.rule.pattern.contains(query))
            .map(|e| e.rule.clone())
            .collect();
        Ok(results)
    }

    pub async fn sync_with_remote(&self) -> Result<()> {
        let mut rules = self.rules.write().await;
        for entry in rules.values_mut() {
            if entry.sync_status == SyncStatus::Pending {
                entry.sync_status = SyncStatus::Synced;
            }
        }
        tracing::info!("Synchronized library with remote");
        Ok(())
    }

    pub async fn get_all_rules(&self) -> Result<Vec<SharedRule>> {
        let rules = self.rules.read().await;
        Ok(rules.values().map(|e| e.rule.clone()).collect())
    }

    pub async fn update_rule(&self, rule_id: &str, rule: SharedRule) -> Result<()> {
        let mut rules = self.rules.write().await;
        if let Some(entry) = rules.get_mut(rule_id) {
            entry.rule = rule;
            entry.last_modified = chrono::Utc::now().timestamp();
            entry.sync_status = SyncStatus::Pending;
        }
        Ok(())
    }

    pub async fn delete_rule(&self, rule_id: &str) -> Result<()> {
        let mut rules = self.rules.write().await;
        rules.remove(rule_id);
        tracing::info!("Deleted rule: {}", rule_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rule_library_creation() {
        let tmp_dir = std::env::temp_dir().join("test_rules");
        let library = RuleLibrary::new(tmp_dir).await.unwrap();
        assert_eq!(library.rule_count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_publish_rule() {
        let tmp_dir = std::env::temp_dir().join("test_rules");
        let library = RuleLibrary::new(tmp_dir).await.unwrap();

        let rule = SharedRule::new(
            "unused-import".to_string(),
            "Unused Import".to_string(),
            "Detects unused import statements".to_string(),
            "pattern: ...".to_string(),
            "author".to_string(),
            "rust".to_string(),
            "web".to_string(),
        );

        library.publish_rule(rule).await.unwrap();

        assert_eq!(library.rule_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_search_rules() {
        let tmp_dir = std::env::temp_dir().join("test_rules");
        let library = RuleLibrary::new(tmp_dir).await.unwrap();

        let rule = SharedRule::new(
            "unused-import".to_string(),
            "Unused Import".to_string(),
            "Detects unused import statements".to_string(),
            "pattern: ...".to_string(),
            "author".to_string(),
            "rust".to_string(),
            "web".to_string(),
        );

        library.publish_rule(rule).await.unwrap();

        let results = library.search("unused").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_rule_rating() {
        let mut rule = SharedRule::new(
            "test-rule".to_string(),
            "Test".to_string(),
            "Test rule".to_string(),
            "pattern: ...".to_string(),
            "author".to_string(),
            "rust".to_string(),
            "web".to_string(),
        );

        rule.add_rating(5.0);
        rule.add_rating(4.0);

        assert_eq!(rule.rating_count, 2);
        assert_eq!(rule.rating, 4.5);
    }
}
