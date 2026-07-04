use crate::{BulkheadMetrics, BulkheadPolicy, ResilienceError, ResilienceResult};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct Bulkhead {
    #[allow(dead_code)]
    service_id: String,
    active_calls: Arc<AtomicUsize>,
    queued_calls: Arc<AtomicUsize>,
    policy: BulkheadPolicy,
    metrics: Arc<parking_lot::Mutex<BulkheadMetrics>>,
}

impl Bulkhead {
    pub fn new(service_id: &str, policy: BulkheadPolicy) -> Self {
        Self {
            service_id: service_id.to_string(),
            active_calls: Arc::new(AtomicUsize::new(0)),
            queued_calls: Arc::new(AtomicUsize::new(0)),
            policy,
            metrics: Arc::new(parking_lot::Mutex::new(BulkheadMetrics {
                service_id: service_id.to_string(),
                active_calls: 0,
                queued_calls: 0,
                total_calls: 0,
                rejected_calls: 0,
                timeout_calls: 0,
            })),
        }
    }

    pub async fn acquire_permit(&self) -> ResilienceResult<BulkheadPermit<'_>> {
        let active = self.active_calls.load(Ordering::SeqCst);
        let queued = self.queued_calls.load(Ordering::SeqCst);

        if active >= self.policy.max_concurrent_calls {
            if queued >= self.policy.queue_capacity {
                let mut metrics = self.metrics.lock();
                metrics.rejected_calls += 1;
                return Err(ResilienceError::BulkheadLimitExceeded(active));
            }

            self.queued_calls.fetch_add(1, Ordering::SeqCst);
        }

        self.active_calls.fetch_add(1, Ordering::SeqCst);

        let mut metrics = self.metrics.lock();
        metrics.active_calls = self.active_calls.load(Ordering::SeqCst);
        metrics.queued_calls = self.queued_calls.load(Ordering::SeqCst);
        metrics.total_calls += 1;

        Ok(BulkheadPermit {
            bulkhead: self,
        })
    }

    pub async fn get_metrics(&self) -> ResilienceResult<BulkheadMetrics> {
        let metrics = self.metrics.lock();
        Ok(metrics.clone())
    }

    fn release_permit(&self) {
        let queued = self.queued_calls.load(Ordering::SeqCst);
        if queued > 0 {
            self.queued_calls.fetch_sub(1, Ordering::SeqCst);
        }

        self.active_calls.fetch_sub(1, Ordering::SeqCst);

        let mut metrics = self.metrics.lock();
        metrics.active_calls = self.active_calls.load(Ordering::SeqCst);
    }

    pub fn active_call_count(&self) -> usize {
        self.active_calls.load(Ordering::SeqCst)
    }

    pub fn queued_call_count(&self) -> usize {
        self.queued_calls.load(Ordering::SeqCst)
    }
}

pub struct BulkheadPermit<'a> {
    bulkhead: &'a Bulkhead,
}

impl<'a> Drop for BulkheadPermit<'a> {
    fn drop(&mut self) {
        self.bulkhead.release_permit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_permit() {
        let policy = BulkheadPolicy {
            max_concurrent_calls: 5,
            queue_capacity: 10,
            timeout_duration_ms: 1000,
        };
        let bulkhead = Bulkhead::new("service-1", policy);

        let _permit = bulkhead.acquire_permit().await.unwrap();
        assert_eq!(bulkhead.active_call_count(), 1);
    }

    #[tokio::test]
    async fn test_permit_released_on_drop() {
        let policy = BulkheadPolicy::default();
        let bulkhead = Bulkhead::new("service-1", policy);

        {
            let _permit = bulkhead.acquire_permit().await.unwrap();
            assert_eq!(bulkhead.active_call_count(), 1);
        }

        assert_eq!(bulkhead.active_call_count(), 0);
    }

    #[tokio::test]
    async fn test_bulkhead_limit_exceeded() {
        let policy = BulkheadPolicy {
            max_concurrent_calls: 2,
            queue_capacity: 0,
            timeout_duration_ms: 1000,
        };
        let bulkhead = Bulkhead::new("service-1", policy);

        let _p1 = bulkhead.acquire_permit().await.unwrap();
        let _p2 = bulkhead.acquire_permit().await.unwrap();

        let result = bulkhead.acquire_permit().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_bulkhead_queue() {
        let policy = BulkheadPolicy {
            max_concurrent_calls: 1,
            queue_capacity: 2,
            timeout_duration_ms: 1000,
        };
        let bulkhead = Bulkhead::new("service-1", policy);

        let _p1 = bulkhead.acquire_permit().await.unwrap();
        let _p2 = bulkhead.acquire_permit().await.unwrap();

        assert_eq!(bulkhead.active_call_count(), 2);
        assert_eq!(bulkhead.queued_call_count(), 1);
    }

    #[tokio::test]
    async fn test_get_metrics() {
        let policy = BulkheadPolicy::default();
        let bulkhead = Bulkhead::new("service-1", policy);

        let _permit = bulkhead.acquire_permit().await.unwrap();
        let metrics = bulkhead.get_metrics().await.unwrap();

        assert_eq!(metrics.active_calls, 1);
        assert_eq!(metrics.total_calls, 1);
    }

    #[tokio::test]
    async fn test_multiple_permits() {
        let policy = BulkheadPolicy {
            max_concurrent_calls: 5,
            queue_capacity: 10,
            timeout_duration_ms: 1000,
        };
        let bulkhead = Bulkhead::new("service-1", policy);

        let _p1 = bulkhead.acquire_permit().await.unwrap();
        let _p2 = bulkhead.acquire_permit().await.unwrap();
        let _p3 = bulkhead.acquire_permit().await.unwrap();

        assert_eq!(bulkhead.active_call_count(), 3);

        drop(_p1);
        drop(_p2);

        assert_eq!(bulkhead.active_call_count(), 1);
    }
}
