//! Actor Mailbox — Message Queue
//!
//! FIFO queue for messages sent to an actor.

use std::sync::Arc;
use crossbeam_queue::ArrayQueue;

/// Mailbox for actor message delivery
pub struct Mailbox {
    messages: Arc<ArrayQueue<Vec<u8>>>,
}

impl Mailbox {
    pub fn new(capacity: usize) -> Self {
        Self {
            messages: Arc::new(ArrayQueue::new(capacity)),
        }
    }

    pub fn send(&self, message: Vec<u8>) -> Result<(), Vec<u8>> {
        self.messages.push(message)
    }

    pub fn recv(&self) -> Option<Vec<u8>> {
        self.messages.pop()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

impl Clone for Mailbox {
    fn clone(&self) -> Self {
        Self {
            messages: self.messages.clone(),
        }
    }
}
