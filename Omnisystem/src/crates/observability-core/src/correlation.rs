use crate::{CorrelationContext, CorrelationId, ObservabilityError, ObservabilityResult, SpanId, TraceId};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct CorrelationManager {
    contexts: Arc<DashMap<String, CorrelationContext>>,
}

impl CorrelationManager {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_context(
        &self,
        trace_id: &TraceId,
        span_id: &SpanId,
    ) -> ObservabilityResult<CorrelationId> {
        let correlation_id = CorrelationId(uuid::Uuid::new_v4().to_string());

        let context = CorrelationContext {
            correlation_id: correlation_id.clone(),
            trace_id: trace_id.clone(),
            span_id: span_id.clone(),
            baggage: HashMap::new(),
        };

        self.contexts.insert(correlation_id.0.clone(), context);
        Ok(correlation_id)
    }

    pub async fn get_context(
        &self,
        correlation_id: &CorrelationId,
    ) -> ObservabilityResult<CorrelationContext> {
        self.contexts
            .get(&correlation_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| ObservabilityError::CorrelationIdMissing)
    }

    pub async fn set_baggage(
        &self,
        correlation_id: &CorrelationId,
        key: &str,
        value: &str,
    ) -> ObservabilityResult<()> {
        if let Some(mut context) = self.contexts.get_mut(&correlation_id.0) {
            context.baggage.insert(key.to_string(), value.to_string());
            Ok(())
        } else {
            Err(ObservabilityError::CorrelationIdMissing)
        }
    }

    pub async fn get_baggage(
        &self,
        correlation_id: &CorrelationId,
        key: &str,
    ) -> ObservabilityResult<Option<String>> {
        self.contexts
            .get(&correlation_id.0)
            .map(|entry| entry.baggage.get(key).cloned())
            .ok_or_else(|| ObservabilityError::CorrelationIdMissing)
    }

    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }
}

impl Default for CorrelationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_context() {
        let manager = CorrelationManager::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        let correlation_id = manager.create_context(&trace_id, &span_id).await.unwrap();
        assert_eq!(manager.context_count(), 1);

        let context = manager.get_context(&correlation_id).await.unwrap();
        assert_eq!(context.trace_id, trace_id);
        assert_eq!(context.span_id, span_id);
    }

    #[tokio::test]
    async fn test_set_baggage() {
        let manager = CorrelationManager::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        let correlation_id = manager.create_context(&trace_id, &span_id).await.unwrap();
        manager.set_baggage(&correlation_id, "user_id", "12345").await.unwrap();

        let context = manager.get_context(&correlation_id).await.unwrap();
        assert_eq!(context.baggage.get("user_id"), Some(&"12345".to_string()));
    }

    #[tokio::test]
    async fn test_get_baggage() {
        let manager = CorrelationManager::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        let correlation_id = manager.create_context(&trace_id, &span_id).await.unwrap();
        manager.set_baggage(&correlation_id, "session_id", "abc123").await.unwrap();

        let value = manager.get_baggage(&correlation_id, "session_id").await.unwrap();
        assert_eq!(value, Some("abc123".to_string()));
    }

    #[tokio::test]
    async fn test_get_missing_baggage() {
        let manager = CorrelationManager::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        let correlation_id = manager.create_context(&trace_id, &span_id).await.unwrap();

        let value = manager.get_baggage(&correlation_id, "nonexistent").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_get_nonexistent_context() {
        let manager = CorrelationManager::new();
        let correlation_id = CorrelationId(uuid::Uuid::new_v4().to_string());

        let result = manager.get_context(&correlation_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_baggage_multiple_values() {
        let manager = CorrelationManager::new();
        let trace_id = TraceId(uuid::Uuid::new_v4().to_string());
        let span_id = SpanId(uuid::Uuid::new_v4().to_string());

        let correlation_id = manager.create_context(&trace_id, &span_id).await.unwrap();
        manager.set_baggage(&correlation_id, "key1", "value1").await.unwrap();
        manager.set_baggage(&correlation_id, "key2", "value2").await.unwrap();
        manager.set_baggage(&correlation_id, "key3", "value3").await.unwrap();

        let context = manager.get_context(&correlation_id).await.unwrap();
        assert_eq!(context.baggage.len(), 3);
    }

    #[tokio::test]
    async fn test_set_baggage_on_missing_context() {
        let manager = CorrelationManager::new();
        let correlation_id = CorrelationId(uuid::Uuid::new_v4().to_string());

        let result = manager.set_baggage(&correlation_id, "key", "value").await;
        assert!(result.is_err());
    }
}
