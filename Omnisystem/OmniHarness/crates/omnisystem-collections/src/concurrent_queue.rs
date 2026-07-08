//! Multi-producer, multi-consumer queue

use std::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use std::sync::Arc;
use std::collections::VecDeque;

/// Multi-producer, single-consumer queue sender
pub struct MpscSender<T> {
    inner: Arc<QueueInner<T>>,
}

/// Multi-producer, single-consumer queue receiver
pub struct MpscReceiver<T> {
    inner: Arc<QueueInner<T>>,
}

struct QueueInner<T> {
    queue: std::sync::Mutex<VecDeque<T>>,
    notifier: std::sync::Condvar,
    capacity: usize,
}

/// Create a new MPSC queue
pub fn mpsc_queue<T>(capacity: usize) -> (MpscSender<T>, MpscReceiver<T>) {
    let inner = Arc::new(QueueInner {
        queue: std::sync::Mutex::new(VecDeque::with_capacity(capacity)),
        notifier: std::sync::Condvar::new(),
        capacity,
    });

    let sender = MpscSender {
        inner: Arc::clone(&inner),
    };

    let receiver = MpscReceiver { inner };

    (sender, receiver)
}

impl<T> MpscSender<T> {
    /// Send a value into the queue
    pub fn send(&self, value: T) -> Result<(), T> {
        let mut queue = self.inner.queue.lock().unwrap();

        if queue.len() >= self.inner.capacity {
            return Err(value);
        }

        queue.push_back(value);
        self.inner.notifier.notify_one();
        Ok(())
    }

    /// Try to send without blocking
    pub fn try_send(&self, value: T) -> Result<(), T> {
        self.send(value)
    }

    /// Get the queue length
    pub fn len(&self) -> usize {
        let queue = self.inner.queue.lock().unwrap();
        queue.len()
    }
}

impl<T> Clone for MpscSender<T> {
    fn clone(&self) -> Self {
        MpscSender {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> MpscReceiver<T> {
    /// Receive a value from the queue
    pub fn recv(&self) -> Option<T> {
        let mut queue = self.inner.queue.lock().unwrap();

        while queue.is_empty() {
            queue = self.inner.notifier.wait(queue).unwrap();
        }

        queue.pop_front()
    }

    /// Try to receive without blocking
    pub fn try_recv(&self) -> Option<T> {
        let mut queue = self.inner.queue.lock().unwrap();
        queue.pop_front()
    }

    /// Get the queue length
    pub fn len(&self) -> usize {
        let queue = self.inner.queue.lock().unwrap();
        queue.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        let queue = self.inner.queue.lock().unwrap();
        queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mpsc_queue_send_recv() {
        let (tx, rx) = mpsc_queue::<i32>(10);
        tx.send(42).unwrap();
        assert_eq!(rx.try_recv(), Some(42));
    }

    #[test]
    fn test_mpsc_queue_multiple_sends() {
        let (tx, rx) = mpsc_queue::<i32>(10);
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();

        assert_eq!(rx.try_recv(), Some(1));
        assert_eq!(rx.try_recv(), Some(2));
        assert_eq!(rx.try_recv(), Some(3));
    }

    #[test]
    fn test_mpsc_queue_capacity() {
        let (tx, _rx) = mpsc_queue::<i32>(2);
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        assert_eq!(tx.send(3), Err(3));
    }

    #[test]
    fn test_mpsc_multiple_senders() {
        let (tx, rx) = mpsc_queue::<i32>(10);
        let tx2 = tx.clone();

        tx.send(1).unwrap();
        tx2.send(2).unwrap();

        assert_eq!(rx.try_recv(), Some(1));
        assert_eq!(rx.try_recv(), Some(2));
    }
}
