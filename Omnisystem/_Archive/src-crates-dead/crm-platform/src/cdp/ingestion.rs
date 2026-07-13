//! CDP Data Ingestion Pipeline
//!
//! Handles multi-source customer data collection and normalization.

use super::customer::{Customer, CustomerId, Event};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub enum DataSource {
    WebAnalytics,
    EmailMarketing,
    CRM,
    ExternalApi,
    EventStream,
}

pub struct IngestionConfig {
    pub batch_size: usize,
    pub max_queue_size: usize,
    pub flush_interval_secs: u64,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_queue_size: 10000,
            flush_interval_secs: 60,
        }
    }
}

pub struct RawEvent {
    pub source: DataSource,
    pub customer_id: String,
    pub event_type: String,
    pub timestamp: u64,
    pub properties: HashMap<String, String>,
}

pub struct IngestionPipeline {
    config: IngestionConfig,
    event_queue: Arc<Mutex<Vec<RawEvent>>>,
    customer_store: Arc<Mutex<HashMap<String, Customer>>>,
    processed_count: Arc<Mutex<u64>>,
}

impl IngestionPipeline {
    pub fn new(config: IngestionConfig) -> Self {
        Self {
            config,
            event_queue: Arc::new(Mutex::new(Vec::new())),
            customer_store: Arc::new(Mutex::new(HashMap::new())),
            processed_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Ingest raw event
    pub fn ingest_event(&self, event: RawEvent) -> Result<(), String> {
        let mut queue = self.event_queue.lock();

        if queue.len() >= self.config.max_queue_size {
            return Err("Queue full".to_string());
        }

        queue.push(event);
        Ok(())
    }

    /// Flush queued events to customer store
    pub fn flush(&self) -> Result<usize, String> {
        let mut queue = self.event_queue.lock();
        let mut customers = self.customer_store.lock();
        let mut processed = self.processed_count.lock();

        let batch_size = queue.len().min(self.config.batch_size);
        let to_process: Vec<RawEvent> = queue.drain(..batch_size).collect();

        for event in to_process {
            // Normalize customer ID
            let customer_id = CustomerId::ExternalId(event.customer_id.clone());
            let customer_key = event.customer_id.clone();

            // Get or create customer
            let customer = customers
                .entry(customer_key)
                .or_insert_with(|| Customer::new(customer_id.clone()));

            // Ingest event
            customer.track_event(Event {
                event_type: event.event_type,
                timestamp: event.timestamp,
                properties: event.properties,
            });

            *processed += 1;
        }

        Ok(batch_size)
    }

    /// Flush until empty
    pub fn flush_all(&self) -> Result<usize, String> {
        let mut total = 0;
        loop {
            let flushed = self.flush()?;
            if flushed == 0 {
                break;
            }
            total += flushed;
        }
        Ok(total)
    }

    /// Get customer by ID
    pub fn get_customer(&self, customer_id: &str) -> Option<Customer> {
        let customers = self.customer_store.lock();
        customers.get(customer_id).cloned()
    }

    /// Get all customers
    pub fn list_customers(&self) -> Vec<Customer> {
        let customers = self.customer_store.lock();
        customers.values().cloned().collect()
    }

    /// Get statistics
    pub fn stats(&self) -> IngestionStats {
        let queue = self.event_queue.lock();
        let customers = self.customer_store.lock();
        let processed = self.processed_count.lock();

        IngestionStats {
            queued_events: queue.len(),
            customers_count: customers.len(),
            total_processed: *processed,
        }
    }
}

pub struct IngestionStats {
    pub queued_events: usize,
    pub customers_count: usize,
    pub total_processed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingestion_pipeline_creation() {
        let pipeline = IngestionPipeline::new(IngestionConfig::default());
        let stats = pipeline.stats();
        assert_eq!(stats.queued_events, 0);
        assert_eq!(stats.customers_count, 0);
    }

    #[test]
    fn test_ingest_event() {
        let pipeline = IngestionPipeline::new(IngestionConfig::default());

        let event = RawEvent {
            source: DataSource::WebAnalytics,
            customer_id: "user123".to_string(),
            event_type: "page_view".to_string(),
            timestamp: 1000,
            properties: HashMap::new(),
        };

        assert!(pipeline.ingest_event(event).is_ok());
        let stats = pipeline.stats();
        assert_eq!(stats.queued_events, 1);
    }

    #[test]
    fn test_flush() {
        let pipeline = IngestionPipeline::new(IngestionConfig::default());

        let event = RawEvent {
            source: DataSource::WebAnalytics,
            customer_id: "user123".to_string(),
            event_type: "purchase".to_string(),
            timestamp: 1000,
            properties: HashMap::new(),
        };

        pipeline.ingest_event(event).unwrap();
        let flushed = pipeline.flush().unwrap();
        assert_eq!(flushed, 1);

        let stats = pipeline.stats();
        assert_eq!(stats.queued_events, 0);
        assert_eq!(stats.customers_count, 1);
    }

    #[test]
    fn test_get_customer() {
        let pipeline = IngestionPipeline::new(IngestionConfig::default());

        let event = RawEvent {
            source: DataSource::CRM,
            customer_id: "cust456".to_string(),
            event_type: "signup".to_string(),
            timestamp: 2000,
            properties: HashMap::new(),
        };

        pipeline.ingest_event(event).unwrap();
        pipeline.flush_all().unwrap();

        let customer = pipeline.get_customer("cust456");
        assert!(customer.is_some());
        assert_eq!(customer.unwrap().events.len(), 1);
    }
}
