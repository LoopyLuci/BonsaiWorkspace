/// Message Transport Layer
/// Unified message routing across all systems

use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Transport Configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub max_message_size: usize,
    pub queue_capacity: usize,
    pub timeout_ms: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        TransportConfig {
            max_message_size: 1024 * 1024, // 1MB
            queue_capacity: 10000,
            timeout_ms: 5000,
        }
    }
}

/// System Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub id: String,
    pub from_system: String,
    pub to_system: String,
    pub message_type: String,
    pub priority: MessagePriority,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub requires_ack: bool,
}

/// Message Priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl SystemMessage {
    pub fn new(from: String, to: String, msg_type: String, payload: Vec<u8>) -> Self {
        SystemMessage {
            id: Uuid::new_v4().to_string(),
            from_system: from,
            to_system: to,
            message_type: msg_type,
            priority: MessagePriority::Normal,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            requires_ack: false,
        }
    }

    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_ack_required(mut self) -> Self {
        self.requires_ack = true;
        self
    }
}

/// Message Queue (per system)
pub struct MessageQueue {
    system_id: String,
    messages: Arc<crossbeam::queue::SegQueue<SystemMessage>>,
}

impl MessageQueue {
    pub fn new(system_id: String) -> Self {
        MessageQueue {
            system_id,
            messages: Arc::new(crossbeam::queue::SegQueue::new()),
        }
    }

    pub fn push(&self, message: SystemMessage) -> anyhow::Result<()> {
        self.messages.push(message);
        Ok(())
    }

    pub fn pop(&self) -> Option<SystemMessage> {
        self.messages.pop()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl Clone for MessageQueue {
    fn clone(&self) -> Self {
        MessageQueue {
            system_id: self.system_id.clone(),
            messages: Arc::clone(&self.messages),
        }
    }
}

/// Message Transport - Routes messages between systems
pub struct MessageTransport {
    config: TransportConfig,
    queues: Arc<DashMap<String, MessageQueue>>,
    acknowledgments: Arc<DashMap<String, bool>>,
    stats: Arc<DashMap<String, u64>>,
}

impl MessageTransport {
    pub fn new() -> Self {
        Self::with_config(TransportConfig::default())
    }

    pub fn with_config(config: TransportConfig) -> Self {
        MessageTransport {
            config,
            queues: Arc::new(DashMap::new()),
            acknowledgments: Arc::new(DashMap::new()),
            stats: Arc::new(DashMap::new()),
        }
    }

    pub fn register_system(&self, system_id: String) {
        let queue = MessageQueue::new(system_id.clone());
        self.queues.insert(system_id, queue);
    }

    pub async fn route_message(&self, from: u64, to: u64, payload: Vec<u8>) -> anyhow::Result<()> {
        let from_sys = format!("sys_{}", from);
        let to_sys = format!("sys_{}", to);

        if payload.len() > self.config.max_message_size {
            return Err(anyhow::anyhow!("Message exceeds maximum size"));
        }

        let message = SystemMessage::new(from_sys.clone(), to_sys.clone(), "data".to_string(), payload);

        match self.queues.get(&to_sys) {
            Some(queue) => {
                queue.push(message.clone())?;
                self.stats
                    .entry("messages_routed".to_string())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
                Ok(())
            }
            None => {
                Err(anyhow::anyhow!("Target system not found: {}", to_sys))
            }
        }
    }

    pub fn send_system_message(&self, message: SystemMessage) -> anyhow::Result<()> {
        if message.payload.len() > self.config.max_message_size {
            return Err(anyhow::anyhow!("Message exceeds maximum size"));
        }

        match self.queues.get(&message.to_system) {
            Some(queue) => {
                queue.push(message.clone())?;
                self.stats
                    .entry(format!("msgs_{}", message.message_type))
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
                Ok(())
            }
            None => {
                Err(anyhow::anyhow!("Target system not found"))
            }
        }
    }

    pub fn receive_message(&self, system_id: &str) -> Option<SystemMessage> {
        self.queues.get(system_id).and_then(|queue| queue.pop())
    }

    pub fn acknowledge_message(&self, message_id: &str) {
        self.acknowledgments.insert(message_id.to_string(), true);
    }

    pub fn get_queue_length(&self, system_id: &str) -> usize {
        self.queues
            .get(system_id)
            .map(|q| q.len())
            .unwrap_or(0)
    }

    pub fn stats(&self) -> Vec<(String, u64)> {
        self.stats
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect()
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        // Drain all queues
        for entry in self.queues.iter() {
            while entry.value().pop().is_some() {}
        }
        Ok(())
    }
}

impl Default for MessageTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_message_creation() {
        let msg = SystemMessage::new(
            "sys_1".to_string(),
            "sys_2".to_string(),
            "test".to_string(),
            vec![1, 2, 3],
        );

        assert_eq!(msg.from_system, "sys_1");
        assert_eq!(msg.to_system, "sys_2");
        assert_eq!(msg.priority, MessagePriority::Normal);
    }

    #[test]
    fn test_transport_routing() {
        let transport = MessageTransport::new();
        transport.register_system("sys_1".to_string());
        transport.register_system("sys_2".to_string());

        let msg = SystemMessage::new(
            "sys_1".to_string(),
            "sys_2".to_string(),
            "test".to_string(),
            vec![1, 2, 3],
        );

        assert!(transport.send_system_message(msg.clone()).is_ok());
        assert!(transport.receive_message("sys_2").is_some());
    }

    #[tokio::test]
    async fn test_message_routing() {
        let transport = MessageTransport::new();
        transport.register_system("sys_1".to_string());
        transport.register_system("sys_2".to_string());

        assert!(transport.route_message(1, 2, vec![1, 2, 3]).await.is_ok());
        assert!(transport.receive_message("sys_2").is_some());
    }
}
