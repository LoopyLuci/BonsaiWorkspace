/// Inter-Process Communication (IPC)
/// Message passing, channels, and RPC

use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// IPC Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: u64,
    pub to: u64,
    pub message_type: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

impl Message {
    pub fn new(from: u64, to: u64, message_type: String, payload: Vec<u8>) -> Self {
        Message {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            message_type,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// IPC Channel - Bidirectional communication between processes
pub struct IPCChannel {
    from: u64,
    to: u64,
    messages: Arc<crossbeam::queue::ArrayQueue<Message>>,
}

impl IPCChannel {
    pub fn new(from: u64, to: u64, capacity: usize) -> Self {
        IPCChannel {
            from,
            to,
            messages: Arc::new(crossbeam::queue::ArrayQueue::new(capacity)),
        }
    }

    pub fn send(&self, message: Message) -> anyhow::Result<()> {
        self.messages.push(message)
            .map_err(|_| anyhow::anyhow!("Channel queue full"))
    }

    pub fn receive(&self) -> Option<Message> {
        self.messages.pop()
    }

    pub fn capacity(&self) -> usize {
        self.messages.capacity()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }
}

/// Message Queue - Manages messages for a process
pub struct MessageQueue {
    pid: u64,
    inbox: Arc<crossbeam::queue::ArrayQueue<Message>>,
}

impl MessageQueue {
    pub fn new(pid: u64, capacity: usize) -> Self {
        MessageQueue {
            pid,
            inbox: Arc::new(crossbeam::queue::ArrayQueue::new(capacity)),
        }
    }

    pub fn send(&self, message: Message) -> anyhow::Result<()> {
        self.inbox.push(message)
            .map_err(|_| anyhow::anyhow!("Message queue full"))
    }

    pub fn receive(&self) -> Option<Message> {
        self.inbox.pop()
    }

    pub fn receive_by_type(&self, message_type: &str) -> Option<Message> {
        // This is a simplified version - real implementation would scan queue
        self.receive()
    }

    pub fn pending_count(&self) -> usize {
        self.inbox.len()
    }
}

/// IPC Manager - Global message routing
pub struct IPCManager {
    message_queues: Arc<DashMap<u64, MessageQueue>>,
    channels: Arc<DashMap<(u64, u64), IPCChannel>>,
}

impl IPCManager {
    pub fn new() -> Self {
        IPCManager {
            message_queues: Arc::new(DashMap::new()),
            channels: Arc::new(DashMap::new()),
        }
    }

    pub fn create_queue(&self, pid: u64, capacity: usize) -> MessageQueue {
        let queue = MessageQueue::new(pid, capacity);
        self.message_queues.insert(pid, queue.clone());
        queue
    }

    pub fn get_queue(&self, pid: u64) -> Option<MessageQueue> {
        self.message_queues.get(&pid).map(|q| q.clone())
    }

    pub fn create_channel(&self, from: u64, to: u64, capacity: usize) -> anyhow::Result<IPCChannel> {
        let channel = IPCChannel::new(from, to, capacity);
        self.channels.insert((from, to), channel.clone());
        Ok(channel)
    }

    pub fn send_message(&self, message: Message) -> anyhow::Result<()> {
        // Try to send via channel first
        if let Some(channel) = self.channels.get(&(message.from, message.to)) {
            return channel.send(message);
        }

        // Fall back to message queue
        if let Some(queue) = self.get_queue(message.to) {
            return queue.send(message);
        }

        Err(anyhow::anyhow!("No route to process {}", message.to))
    }

    pub fn receive_message(&self, pid: u64) -> Option<Message> {
        self.get_queue(pid).and_then(|q| q.receive())
    }

    pub fn broadcast(&self, from: u64, message_type: String, payload: Vec<u8>) -> usize {
        let mut sent_count = 0;

        for entry in self.message_queues.iter() {
            let pid = entry.key().clone();
            if pid != from {
                let msg = Message::new(from, pid, message_type.clone(), payload.clone());
                if entry.value().send(msg).is_ok() {
                    sent_count += 1;
                }
            }
        }

        sent_count
    }
}

impl Default for IPCManager {
    fn default() -> Self {
        Self::new()
    }
}

// Helper: Make MessageQueue cloneable
impl Clone for MessageQueue {
    fn clone(&self) -> Self {
        MessageQueue {
            pid: self.pid,
            inbox: Arc::clone(&self.inbox),
        }
    }
}

// Helper: Make IPCChannel cloneable
impl Clone for IPCChannel {
    fn clone(&self) -> Self {
        IPCChannel {
            from: self.from,
            to: self.to,
            messages: Arc::clone(&self.messages),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(1, 2, "test".to_string(), vec![1, 2, 3]);
        assert_eq!(msg.from, 1);
        assert_eq!(msg.to, 2);
        assert_eq!(msg.payload.len(), 3);
    }

    #[test]
    fn test_message_queue() {
        let queue = MessageQueue::new(1, 10);
        let msg = Message::new(0, 1, "hello".to_string(), vec![]);

        queue.send(msg.clone()).unwrap();
        assert_eq!(queue.pending_count(), 1);

        let received = queue.receive().unwrap();
        assert_eq!(received.message_type, "hello");
        assert_eq!(queue.pending_count(), 0);
    }

    #[test]
    fn test_ipc_manager() {
        let manager = IPCManager::new();
        manager.create_queue(1, 10);
        manager.create_queue(2, 10);

        let msg = Message::new(1, 2, "test".to_string(), vec![42]);
        manager.send_message(msg).unwrap();

        let received = manager.receive_message(2).unwrap();
        assert_eq!(received.from, 1);
        assert_eq!(received.to, 2);
    }

    #[test]
    fn test_broadcast() {
        let manager = IPCManager::new();
        manager.create_queue(1, 10);
        manager.create_queue(2, 10);
        manager.create_queue(3, 10);

        let count = manager.broadcast(1, "broadcast".to_string(), vec![]);
        assert_eq!(count, 2); // Sent to processes 2 and 3
    }
}
