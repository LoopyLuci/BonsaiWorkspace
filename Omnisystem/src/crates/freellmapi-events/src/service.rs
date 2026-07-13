use crate::{EventLog, EventRecord, EventType, WebhookDelivery, WebhookStatus};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use freellmapi_core::OmnisystemService;
use std::sync::Arc;

pub struct EventService {
    event_log: Arc<dyn EventLog>,
    webhooks: Arc<DashMap<String, Vec<String>>>,
    deliveries: Arc<DashMap<String, WebhookDelivery>>,
}

impl EventService {
    pub async fn new() -> Result<Self> {
        let event_log = Arc::new(crate::event_log::InMemoryEventLog::new());

        Ok(EventService {
            event_log,
            webhooks: Arc::new(DashMap::new()),
            deliveries: Arc::new(DashMap::new()),
        })
    }

    pub async fn log_event(&self, record: &EventRecord) -> Result<()> {
        self.event_log.append(record).await?;

        // Trigger webhook delivery if webhooks registered for this tenant
        if let Some(urls) = self.webhooks.get(&record.tenant_id) {
            for url in urls.iter() {
                let delivery = WebhookDelivery {
                    id: uuid::Uuid::new_v4().to_string(),
                    tenant_id: record.tenant_id.clone(),
                    webhook_url: url.clone(),
                    event_id: record.id.clone(),
                    payload: record.data.clone(),
                    status: WebhookStatus::Pending,
                    attempts: 0,
                    max_retries: 3,
                    created_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    last_attempt_at: None,
                };

                self.deliveries.insert(delivery.id.clone(), delivery.clone());
                // In a real implementation, spawn async task to deliver webhook
            }
        }

        Ok(())
    }

    pub async fn get_events(&self, tenant_id: &str, limit: usize) -> Result<Vec<EventRecord>> {
        self.event_log.query(tenant_id, limit).await
    }

    pub async fn get_events_by_type(&self, tenant_id: &str, event_type: EventType, limit: usize) -> Result<Vec<EventRecord>> {
        self.event_log.query_by_type(tenant_id, event_type, limit).await
    }

    pub async fn register_webhook(&self, tenant_id: &str, webhook_url: &str) -> Result<()> {
        let mut entry = self.webhooks
            .entry(tenant_id.to_string())
            .or_insert_with(Vec::new);
        entry.push(webhook_url.to_string());
        Ok(())
    }

    pub async fn get_webhooks(&self, tenant_id: &str) -> Result<Vec<String>> {
        if let Some(entry) = self.webhooks.get(tenant_id) {
            Ok(entry.value().clone())
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_delivery_status(&self, delivery_id: &str) -> Result<Option<WebhookStatus>> {
        if let Some(delivery) = self.deliveries.get(delivery_id) {
            Ok(Some(delivery.value().status.clone()))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl OmnisystemService for EventService {
    fn service_id(&self) -> &str {
        "freellmapi-events"
    }

    fn service_name(&self) -> &str {
        "FreeLLMAPI Events System"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    async fn initialize(&self) -> Result<()> {
        tracing::info!("Event service initialized");
        Ok(())
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_get_webhooks() {
        let service = EventService::new().await.unwrap();
        service.register_webhook("tenant1", "https://example.com/webhook").await.unwrap();

        let webhooks = service.get_webhooks("tenant1").await.unwrap();
        assert_eq!(webhooks.len(), 1);
        assert_eq!(webhooks[0], "https://example.com/webhook");
    }

    #[tokio::test]
    async fn test_event_triggers_webhook_registration() {
        let service = EventService::new().await.unwrap();
        service.register_webhook("tenant1", "https://example.com/webhook").await.unwrap();

        let record = EventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::RequestCompleted,
            tenant_id: "tenant1".to_string(),
            data: serde_json::json!({"status": "ok"}),
            timestamp: 1000,
        };

        service.log_event(&record).await.unwrap();

        let events = service.get_events("tenant1", 10).await.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_get_events_by_type() {
        let service = EventService::new().await.unwrap();

        let record1 = EventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::RequestStarted,
            tenant_id: "tenant1".to_string(),
            data: serde_json::json!({}),
            timestamp: 1000,
        };

        let record2 = EventRecord {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: EventType::RequestCompleted,
            tenant_id: "tenant1".to_string(),
            data: serde_json::json!({}),
            timestamp: 2000,
        };

        service.log_event(&record1).await.unwrap();
        service.log_event(&record2).await.unwrap();

        let started = service.get_events_by_type("tenant1", EventType::RequestStarted, 10).await.unwrap();
        assert_eq!(started.len(), 1);

        let completed = service.get_events_by_type("tenant1", EventType::RequestCompleted, 10).await.unwrap();
        assert_eq!(completed.len(), 1);
    }
}
