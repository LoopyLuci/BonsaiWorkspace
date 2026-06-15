use crate::{RateLimitResult, RequestPriority};
use dashmap::DashMap;
use std::sync::Arc;

pub struct PriorityQueueManager {
    queues: Arc<DashMap<String, Vec<(RequestPriority, String)>>>,
}

impl PriorityQueueManager {
    pub fn new() -> Self {
        Self {
            queues: Arc::new(DashMap::new()),
        }
    }

    pub async fn enqueue(
        &self,
        queue_id: &str,
        request_id: &str,
        priority: RequestPriority,
    ) -> RateLimitResult<()> {
        let mut queue = self
            .queues
            .entry(queue_id.to_string())
            .or_insert_with(Vec::new);

        queue.push((priority, request_id.to_string()));
        queue.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(())
    }

    pub async fn dequeue(&self, queue_id: &str) -> RateLimitResult<Option<String>> {
        if let Some(mut queue) = self.queues.get_mut(queue_id) {
            if queue.is_empty() {
                Ok(None)
            } else {
                Ok(Some(queue.remove(0).1))
            }
        } else {
            Ok(None)
        }
    }

    pub async fn peek(&self, queue_id: &str) -> RateLimitResult<Option<(RequestPriority, String)>> {
        if let Some(queue) = self.queues.get(queue_id) {
            Ok(queue.last().cloned())
        } else {
            Ok(None)
        }
    }

    pub async fn get_queue_length(&self, queue_id: &str) -> RateLimitResult<usize> {
        if let Some(queue) = self.queues.get(queue_id) {
            Ok(queue.len())
        } else {
            Ok(0)
        }
    }

    pub async fn clear_queue(&self, queue_id: &str) -> RateLimitResult<()> {
        if let Some(mut queue) = self.queues.get_mut(queue_id) {
            queue.clear();
            Ok(())
        } else {
            self.queues.insert(queue_id.to_string(), Vec::new());
            Ok(())
        }
    }

    pub async fn get_priority_distribution(&self, queue_id: &str) -> RateLimitResult<Vec<(RequestPriority, usize)>> {
        if let Some(queue) = self.queues.get(queue_id) {
            let mut distribution = vec![
                (RequestPriority::Critical, 0),
                (RequestPriority::High, 0),
                (RequestPriority::Normal, 0),
                (RequestPriority::Low, 0),
            ];

            for (priority, _) in queue.iter() {
                for entry in &mut distribution {
                    if entry.0 == *priority {
                        entry.1 += 1;
                    }
                }
            }

            Ok(distribution)
        } else {
            Ok(vec![
                (RequestPriority::Critical, 0),
                (RequestPriority::High, 0),
                (RequestPriority::Normal, 0),
                (RequestPriority::Low, 0),
            ])
        }
    }

    pub fn queue_count(&self) -> usize {
        self.queues.len()
    }
}

impl Default for PriorityQueueManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_request() {
        let manager = PriorityQueueManager::new();
        manager
            .enqueue("queue-1", "req-1", RequestPriority::Normal)
            .await
            .unwrap();

        let length = manager.get_queue_length("queue-1").await.unwrap();
        assert_eq!(length, 1);
    }

    #[tokio::test]
    async fn test_dequeue_request() {
        let manager = PriorityQueueManager::new();
        manager
            .enqueue("queue-1", "req-1", RequestPriority::Normal)
            .await
            .unwrap();

        let request = manager.dequeue("queue-1").await.unwrap();
        assert_eq!(request, Some("req-1".to_string()));
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let manager = PriorityQueueManager::new();
        manager
            .enqueue("queue-1", "req-low", RequestPriority::Low)
            .await
            .unwrap();
        manager
            .enqueue("queue-1", "req-high", RequestPriority::High)
            .await
            .unwrap();
        manager
            .enqueue("queue-1", "req-critical", RequestPriority::Critical)
            .await
            .unwrap();

        let first = manager.dequeue("queue-1").await.unwrap();
        assert_eq!(first, Some("req-critical".to_string()));

        let second = manager.dequeue("queue-1").await.unwrap();
        assert_eq!(second, Some("req-high".to_string()));

        let third = manager.dequeue("queue-1").await.unwrap();
        assert_eq!(third, Some("req-low".to_string()));
    }

    #[tokio::test]
    async fn test_peek() {
        let manager = PriorityQueueManager::new();
        manager
            .enqueue("queue-1", "req-1", RequestPriority::Normal)
            .await
            .unwrap();

        let peek = manager.peek("queue-1").await.unwrap();
        assert_eq!(peek, Some((RequestPriority::Normal, "req-1".to_string())));

        let length = manager.get_queue_length("queue-1").await.unwrap();
        assert_eq!(length, 1);
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let manager = PriorityQueueManager::new();
        manager
            .enqueue("queue-1", "req-1", RequestPriority::Normal)
            .await
            .unwrap();
        manager
            .enqueue("queue-1", "req-2", RequestPriority::Normal)
            .await
            .unwrap();

        manager.clear_queue("queue-1").await.unwrap();
        let length = manager.get_queue_length("queue-1").await.unwrap();

        assert_eq!(length, 0);
    }

    #[tokio::test]
    async fn test_priority_distribution() {
        let manager = PriorityQueueManager::new();
        manager
            .enqueue("queue-1", "req-1", RequestPriority::Low)
            .await
            .unwrap();
        manager
            .enqueue("queue-1", "req-2", RequestPriority::High)
            .await
            .unwrap();
        manager
            .enqueue("queue-1", "req-3", RequestPriority::High)
            .await
            .unwrap();

        let distribution = manager.get_priority_distribution("queue-1").await.unwrap();
        assert_eq!(distribution[0].1, 0); // Critical
        assert_eq!(distribution[1].1, 2); // High
        assert_eq!(distribution[2].1, 0); // Normal
        assert_eq!(distribution[3].1, 1); // Low
    }
}
