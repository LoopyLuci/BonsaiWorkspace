use crate::{ContextPropagationConfig, ContextPropagationType, DistributedContext, HealthError, HealthResult};
use chrono::Utc;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ContextPropagator {
    contexts: Arc<DashMap<String, DistributedContext>>,
    config: ContextPropagationConfig,
}

impl ContextPropagator {
    pub fn new(config: ContextPropagationConfig) -> Self {
        Self {
            contexts: Arc::new(DashMap::new()),
            config,
        }
    }

    pub async fn create_context(
        &self,
        trace_id: &str,
        span_id: &str,
        correlation_id: &str,
    ) -> HealthResult<DistributedContext> {
        if !self.config.enabled {
            return Err(HealthError::ContextCorrelationFailed);
        }

        let context = DistributedContext {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            correlation_id: correlation_id.to_string(),
            parent_span_id: None,
            baggage: HashMap::new(),
            timestamp: Utc::now(),
            propagation_type: self.config.propagation_types.first().copied().unwrap_or(ContextPropagationType::TraceContext),
        };

        self.contexts.insert(correlation_id.to_string(), context.clone());
        Ok(context)
    }

    pub async fn get_context(&self, correlation_id: &str) -> HealthResult<DistributedContext> {
        self.contexts
            .get(correlation_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| HealthError::SpanNotFound(correlation_id.to_string()))
    }

    pub async fn add_baggage(
        &self,
        correlation_id: &str,
        key: &str,
        value: &str,
    ) -> HealthResult<()> {
        if !self.config.include_baggage {
            return Err(HealthError::ContextCorrelationFailed);
        }

        if let Some(mut context) = self.contexts.get_mut(correlation_id) {
            let baggage_size: usize = context.baggage.iter().map(|(k, v)| k.len() + v.len()).sum();
            if baggage_size + key.len() + value.len() > self.config.max_baggage_size {
                return Err(HealthError::Internal("Baggage size limit exceeded".to_string()));
            }

            context.baggage.insert(key.to_string(), value.to_string());
            Ok(())
        } else {
            Err(HealthError::SpanNotFound(correlation_id.to_string()))
        }
    }

    pub async fn get_baggage(&self, correlation_id: &str, key: &str) -> HealthResult<Option<String>> {
        self.contexts
            .get(correlation_id)
            .map(|context| context.baggage.get(key).cloned())
            .ok_or_else(|| HealthError::SpanNotFound(correlation_id.to_string()))
    }

    pub async fn create_child_context(
        &self,
        parent_correlation_id: &str,
        new_span_id: &str,
    ) -> HealthResult<DistributedContext> {
        if let Some(parent) = self.contexts.get(parent_correlation_id) {
            let mut child = parent.clone();
            child.parent_span_id = Some(child.span_id.clone());
            child.span_id = new_span_id.to_string();
            child.timestamp = Utc::now();

            self.contexts
                .insert(format!("{}-child", parent_correlation_id), child.clone());

            Ok(child)
        } else {
            Err(HealthError::SpanNotFound(parent_correlation_id.to_string()))
        }
    }

    pub async fn extract_headers(&self, correlation_id: &str) -> HealthResult<HashMap<String, String>> {
        if let Some(context) = self.contexts.get(correlation_id) {
            let mut headers = HashMap::new();

            if self.config.propagation_types.contains(&ContextPropagationType::TraceContext) {
                headers.insert("trace-id".to_string(), context.trace_id.clone());
                headers.insert("span-id".to_string(), context.span_id.clone());
            }

            if self.config.propagation_types.contains(&ContextPropagationType::Correlation) {
                headers.insert("correlation-id".to_string(), context.correlation_id.clone());
            }

            if self.config.include_baggage && !context.baggage.is_empty() {
                for (key, value) in &context.baggage {
                    headers.insert(format!("baggage-{}", key), value.clone());
                }
            }

            Ok(headers)
        } else {
            Err(HealthError::SpanNotFound(correlation_id.to_string()))
        }
    }

    pub async fn inject_headers(
        &self,
        correlation_id: &str,
        headers: &mut HashMap<String, String>,
    ) -> HealthResult<()> {
        let extracted = self.extract_headers(correlation_id).await?;
        headers.extend(extracted);
        Ok(())
    }

    pub fn context_count(&self) -> usize {
        self.contexts.len()
    }
}

impl Default for ContextPropagator {
    fn default() -> Self {
        Self::new(ContextPropagationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_context() {
        let propagator = ContextPropagator::default();
        let context = propagator
            .create_context("trace-123", "span-456", "corr-789")
            .await
            .unwrap();

        assert_eq!(context.trace_id, "trace-123");
        assert_eq!(context.span_id, "span-456");
        assert_eq!(context.correlation_id, "corr-789");
    }

    #[tokio::test]
    async fn test_get_context() {
        let propagator = ContextPropagator::default();
        propagator
            .create_context("trace-123", "span-456", "corr-789")
            .await
            .unwrap();

        let context = propagator.get_context("corr-789").await.unwrap();
        assert_eq!(context.trace_id, "trace-123");
    }

    #[tokio::test]
    async fn test_add_baggage() {
        let propagator = ContextPropagator::default();
        propagator
            .create_context("trace-123", "span-456", "corr-789")
            .await
            .unwrap();

        propagator
            .add_baggage("corr-789", "user-id", "user-123")
            .await
            .unwrap();

        let baggage = propagator.get_baggage("corr-789", "user-id").await.unwrap();
        assert_eq!(baggage, Some("user-123".to_string()));
    }

    #[tokio::test]
    async fn test_create_child_context() {
        let propagator = ContextPropagator::default();
        propagator
            .create_context("trace-123", "span-456", "corr-789")
            .await
            .unwrap();

        let child = propagator
            .create_child_context("corr-789", "span-999")
            .await
            .unwrap();

        assert_eq!(child.parent_span_id, Some("span-456".to_string()));
        assert_eq!(child.span_id, "span-999");
        assert_eq!(child.trace_id, "trace-123");
    }

    #[tokio::test]
    async fn test_extract_headers() {
        let propagator = ContextPropagator::default();
        propagator
            .create_context("trace-123", "span-456", "corr-789")
            .await
            .unwrap();

        let headers = propagator.extract_headers("corr-789").await.unwrap();
        assert_eq!(headers.get("trace-id").unwrap(), "trace-123");
        assert_eq!(headers.get("span-id").unwrap(), "span-456");
    }

    #[tokio::test]
    async fn test_inject_headers() {
        let propagator = ContextPropagator::default();
        propagator
            .create_context("trace-123", "span-456", "corr-789")
            .await
            .unwrap();

        let mut headers = HashMap::new();
        propagator.inject_headers("corr-789", &mut headers).await.unwrap();

        assert!(!headers.is_empty());
        assert!(headers.contains_key("trace-id"));
    }

    #[tokio::test]
    async fn test_disabled_context_propagation() {
        let config = ContextPropagationConfig {
            enabled: false,
            ..Default::default()
        };
        let propagator = ContextPropagator::new(config);

        let result = propagator
            .create_context("trace-123", "span-456", "corr-789")
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_baggage_size_limit() {
        let mut config = ContextPropagationConfig::default();
        config.max_baggage_size = 10;

        let propagator = ContextPropagator::new(config);
        propagator
            .create_context("trace-123", "span-456", "corr-789")
            .await
            .unwrap();

        let result = propagator
            .add_baggage("corr-789", "key", "this-is-too-long-value")
            .await;

        assert!(result.is_err());
    }
}
