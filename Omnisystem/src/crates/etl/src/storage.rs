/// ETL Storage - Persistence layer for feedback events and metrics
use crate::confidence::RuleConfidenceMetrics;
use crate::events::FeedbackEvent;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory storage backend (TODO: Replace with SQLx for production)
pub struct ETLStorage {
    feedback_events: Arc<RwLock<Vec<FeedbackEvent>>>,
    metrics_cache: Arc<RwLock<HashMap<String, RuleConfidenceMetrics>>>,
}

impl ETLStorage {
    pub fn new() -> Self {
        Self {
            feedback_events: Arc::new(RwLock::new(Vec::new())),
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a single feedback event
    pub async fn store_feedback_event(&self, event: &FeedbackEvent) -> anyhow::Result<()> {
        let mut events = self.feedback_events.write().await;
        events.push(event.clone());
        tracing::debug!("Stored feedback event: {} for rule {}", event.event_id, event.rule_id);
        Ok(())
    }

    /// Get all feedback events since a given timestamp
    pub async fn get_feedback_events_since(
        &self,
        since: DateTime<Utc>,
    ) -> anyhow::Result<Vec<FeedbackEvent>> {
        let events = self.feedback_events.read().await;
        let filtered = events
            .iter()
            .filter(|e| e.timestamp >= since)
            .cloned()
            .collect();
        Ok(filtered)
    }

    /// Get feedback events for a specific rule
    pub async fn get_feedback_events_for_rule(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<FeedbackEvent>> {
        let events = self.feedback_events.read().await;
        let filtered = events
            .iter()
            .filter(|e| e.rule_id == rule_id)
            .cloned()
            .collect();
        Ok(filtered)
    }

    /// Store aggregated metrics for later retrieval
    pub async fn store_metrics(
        &self,
        metrics: &HashMap<String, RuleConfidenceMetrics>,
    ) -> anyhow::Result<()> {
        let mut cache = self.metrics_cache.write().await;
        for (rule_id, metric) in metrics {
            cache.insert(rule_id.clone(), metric.clone());
        }
        tracing::info!("Stored metrics for {} rules", metrics.len());
        Ok(())
    }

    /// Retrieve cached metrics for a rule
    pub async fn get_metrics(&self, rule_id: &str) -> anyhow::Result<Option<RuleConfidenceMetrics>> {
        let cache = self.metrics_cache.read().await;
        Ok(cache.get(rule_id).cloned())
    }

    /// Clear old feedback events (older than N days)
    pub async fn cleanup_old_events(&self, days_old: i64) -> anyhow::Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(days_old);
        let mut events = self.feedback_events.write().await;
        let before = events.len();
        events.retain(|e| e.timestamp >= cutoff);
        let removed = before - events.len();
        tracing::info!("Cleaned up {} old feedback events", removed);
        Ok(removed)
    }
}

impl Default for ETLStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::FeedbackEventType;

    #[tokio::test]
    async fn test_store_and_retrieve_feedback() {
        let storage = ETLStorage::new();
        let event = FeedbackEvent {
            event_id: "test1".to_string(),
            event_type: FeedbackEventType::DiagnosticAccepted,
            rule_id: "rule-1".to_string(),
            file: "test.rs".to_string(),
            line: 42,
            timestamp: Utc::now(),
            user_id: "user-1".to_string(),
            action: Some("apply".to_string()),
            outcome: Some("success".to_string()),
            explanation: None,
            dismissal_count: None,
        };

        storage.store_feedback_event(&event).await.unwrap();

        let since = Utc::now() - chrono::Duration::hours(1);
        let events = storage.get_feedback_events_since(since).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].rule_id, "rule-1");
    }

    #[tokio::test]
    async fn test_store_metrics() {
        let storage = ETLStorage::new();
        let mut metrics = HashMap::new();
        let metric = RuleConfidenceMetrics {
            rule_id: "rule-1".to_string(),
            true_positives: 100,
            false_positives: 10,
            dismissed_count: 5,
            applied_fixes: 100,
            fix_success_rate: 0.95,
            last_updated: Utc::now(),
        };

        metrics.insert("rule-1".to_string(), metric);
        storage.store_metrics(&metrics).await.unwrap();

        let retrieved = storage.get_metrics("rule-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().true_positives, 100);
    }

    #[tokio::test]
    async fn test_cleanup_old_events() {
        let storage = ETLStorage::new();

        let event = FeedbackEvent {
            event_id: "old".to_string(),
            event_type: FeedbackEventType::DiagnosticAccepted,
            rule_id: "rule-1".to_string(),
            file: "test.rs".to_string(),
            line: 42,
            timestamp: Utc::now() - chrono::Duration::days(100),
            user_id: "user-1".to_string(),
            action: None,
            outcome: None,
            explanation: None,
            dismissal_count: None,
        };

        storage.store_feedback_event(&event).await.unwrap();
        let removed = storage.cleanup_old_events(30).await.unwrap();
        assert_eq!(removed, 1);

        let remaining = storage.get_feedback_events_since(Utc::now() - chrono::Duration::days(200)).await.unwrap();
        assert_eq!(remaining.len(), 0);
    }
}
