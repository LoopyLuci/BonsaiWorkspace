/// Polyglot Message Bus
/// Unified messaging system for cross-language communication
/// Enables seamless data flow between all 750 language modules

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Message type for inter-language communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyglotMessage {
    pub id: String,
    pub from_language: String,
    pub to_language: String,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
    pub priority: MessagePriority,
}

/// Message priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Message delivery status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    Acknowledged,
}

/// Message Bus - routes messages between language modules
pub struct MessageBus {
    // Queue for each language module
    queues: Arc<DashMap<String, crossbeam::queue::SegQueue<PolyglotMessage>>>,
    // Message history for debugging
    history: Arc<DashMap<String, Vec<PolyglotMessage>>>,
    // Delivery status tracking
    status: Arc<DashMap<String, DeliveryStatus>>,
}

impl MessageBus {
    pub fn new() -> Self {
        MessageBus {
            queues: Arc::new(DashMap::new()),
            history: Arc::new(DashMap::new()),
            status: Arc::new(DashMap::new()),
        }
    }

    /// Register a language module to receive messages
    pub fn register_language(&self, language_id: &str) {
        self.queues.insert(
            language_id.to_string(),
            crossbeam::queue::SegQueue::new(),
        );
    }

    /// Send message from one language to another
    pub fn send_message(
        &self,
        from_language: &str,
        to_language: &str,
        message_type: &str,
        payload: serde_json::Value,
    ) -> anyhow::Result<String> {
        let message_id = Uuid::new_v4().to_string();

        let message = PolyglotMessage {
            id: message_id.clone(),
            from_language: from_language.to_string(),
            to_language: to_language.to_string(),
            message_type: message_type.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
            priority: MessagePriority::Normal,
        };

        // Get or create queue for target language
        if !self.queues.contains_key(to_language) {
            self.register_language(to_language);
        }

        // Add to queue
        let queue = self.queues.get(to_language)
            .ok_or_else(|| anyhow::anyhow!("Language {} not registered", to_language))?;
        queue.push(message.clone());

        // Track in history
        self.history
            .entry(message_id.clone())
            .or_insert_with(Vec::new)
            .push(message);

        // Update status
        self.status.insert(message_id.clone(), DeliveryStatus::Pending);

        tracing::debug!(
            "Message sent from {} to {}: {}",
            from_language,
            to_language,
            message_id
        );

        Ok(message_id)
    }

    /// Receive next message for a language
    pub fn receive_message(&self, language_id: &str) -> Option<PolyglotMessage> {
        self.queues
            .get(language_id)
            .and_then(|queue| queue.pop())
    }

    /// Get all messages in queue for a language
    pub fn peek_queue(&self, language_id: &str) -> Vec<PolyglotMessage> {
        let mut messages = Vec::new();
        if let Some(queue) = self.queues.get(language_id) {
            while let Some(msg) = queue.pop() {
                messages.push(msg);
            }
            // Re-insert if needed (peeking doesn't consume)
            for msg in messages.iter().rev() {
                queue.push(msg.clone());
            }
        }
        messages
    }

    /// Mark message as delivered
    pub fn acknowledge_message(&self, message_id: &str) -> anyhow::Result<()> {
        self.status.insert(
            message_id.to_string(),
            DeliveryStatus::Acknowledged,
        );
        Ok(())
    }

    /// Get message delivery status
    pub fn get_status(&self, message_id: &str) -> Option<DeliveryStatus> {
        self.status.get(message_id).map(|s| *s)
    }

    /// Get message history
    pub fn get_history(&self, message_id: &str) -> Option<Vec<PolyglotMessage>> {
        self.history.get(message_id).map(|h| h.clone())
    }

    /// Get queue depth for a language
    pub fn queue_depth(&self, language_id: &str) -> usize {
        self.queues
            .get(language_id)
            .map(|queue| {
                // Approximate depth (SegQueue doesn't expose len)
                let mut count = 0;
                while let Some(msg) = queue.pop() {
                    count += 1;
                    queue.push(msg);
                }
                count
            })
            .unwrap_or(0)
    }

    /// Clear all messages for a language
    pub fn clear_queue(&self, language_id: &str) {
        if let Some(_) = self.queues.get(language_id) {
            self.queues.remove(language_id);
            self.register_language(language_id);
        }
    }

    /// Get all registered languages
    pub fn registered_languages(&self) -> Vec<String> {
        self.queues.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Shutdown message bus
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.queues.clear();
        self.history.clear();
        self.status.clear();
        tracing::info!("Message bus shutdown complete");
        Ok(())
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_bus() {
        let bus = MessageBus::new();

        bus.register_language("assembly");
        bus.register_language("fortran");

        let payload = serde_json::json!({"data": "test"});
        let msg_id = bus
            .send_message("assembly", "fortran", "compute", payload)
            .unwrap();

        assert!(bus.get_status(&msg_id).is_some());

        let msg = bus.receive_message("fortran");
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().from_language, "assembly");
    }

    #[test]
    fn test_queue_depth() {
        let bus = MessageBus::new();
        bus.register_language("test");

        for i in 0..5 {
            let payload = serde_json::json!({"index": i});
            let _ = bus.send_message("source", "test", "message", payload);
        }

        assert_eq!(bus.queue_depth("test"), 5);
    }
}
