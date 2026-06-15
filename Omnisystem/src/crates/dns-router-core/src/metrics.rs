use crate::{DnsMetrics, DnsResult, MetricsCollector};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct DefaultMetricsCollector {
    total_queries: Arc<AtomicU64>,
    successful_queries: Arc<AtomicU64>,
    failed_queries: Arc<AtomicU64>,
    response_times: Arc<DashMap<u64, f64>>,
}

impl DefaultMetricsCollector {
    pub fn new() -> Self {
        Self {
            total_queries: Arc::new(AtomicU64::new(0)),
            successful_queries: Arc::new(AtomicU64::new(0)),
            failed_queries: Arc::new(AtomicU64::new(0)),
            response_times: Arc::new(DashMap::new()),
        }
    }

    fn calculate_average_response_time(&self) -> f64 {
        if self.response_times.is_empty() {
            return 0.0;
        }

        let sum: f64 = self
            .response_times
            .iter()
            .map(|entry| *entry.value())
            .sum();

        sum / self.response_times.len() as f64
    }

    pub fn response_time_count(&self) -> usize {
        self.response_times.len()
    }
}

impl Default for DefaultMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MetricsCollector for DefaultMetricsCollector {
    async fn record_query(&self, success: bool, response_time_ms: f64) -> DnsResult<()> {
        self.total_queries.fetch_add(1, Ordering::Relaxed);

        if success {
            self.successful_queries.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_queries.fetch_add(1, Ordering::Relaxed);
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        self.response_times.insert(timestamp, response_time_ms);

        Ok(())
    }

    async fn get_metrics(&self) -> DnsResult<DnsMetrics> {
        Ok(DnsMetrics {
            total_queries: self.total_queries.load(Ordering::Relaxed),
            successful_queries: self.successful_queries.load(Ordering::Relaxed),
            failed_queries: self.failed_queries.load(Ordering::Relaxed),
            average_response_time_ms: self.calculate_average_response_time(),
            zones_count: 0,
            records_count: 0,
        })
    }

    async fn reset_metrics(&self) -> DnsResult<()> {
        self.total_queries.store(0, Ordering::Relaxed);
        self.successful_queries.store(0, Ordering::Relaxed);
        self.failed_queries.store(0, Ordering::Relaxed);
        self.response_times.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_successful_query() {
        let collector = DefaultMetricsCollector::new();
        collector.record_query(true, 10.5).await.unwrap();

        let metrics = collector.get_metrics().await.unwrap();
        assert_eq!(metrics.total_queries, 1);
        assert_eq!(metrics.successful_queries, 1);
        assert_eq!(metrics.failed_queries, 0);
    }

    #[tokio::test]
    async fn test_record_failed_query() {
        let collector = DefaultMetricsCollector::new();
        collector.record_query(false, 5.2).await.unwrap();

        let metrics = collector.get_metrics().await.unwrap();
        assert_eq!(metrics.total_queries, 1);
        assert_eq!(metrics.successful_queries, 0);
        assert_eq!(metrics.failed_queries, 1);
    }

    #[tokio::test]
    async fn test_average_response_time() {
        let collector = DefaultMetricsCollector::new();
        collector.record_query(true, 10.0).await.unwrap();
        collector.record_query(true, 20.0).await.unwrap();
        collector.record_query(true, 30.0).await.unwrap();

        let metrics = collector.get_metrics().await.unwrap();
        assert_eq!(metrics.average_response_time_ms, 20.0);
    }

    #[tokio::test]
    async fn test_reset_metrics() {
        let collector = DefaultMetricsCollector::new();
        collector.record_query(true, 10.0).await.unwrap();
        collector.record_query(true, 20.0).await.unwrap();

        let metrics = collector.get_metrics().await.unwrap();
        assert_eq!(metrics.total_queries, 2);

        collector.reset_metrics().await.unwrap();

        let metrics = collector.get_metrics().await.unwrap();
        assert_eq!(metrics.total_queries, 0);
        assert_eq!(metrics.successful_queries, 0);
        assert_eq!(metrics.failed_queries, 0);
    }

    #[tokio::test]
    async fn test_mixed_query_types() {
        let collector = DefaultMetricsCollector::new();
        collector.record_query(true, 15.0).await.unwrap();
        collector.record_query(true, 25.0).await.unwrap();
        collector.record_query(false, 5.0).await.unwrap();

        let metrics = collector.get_metrics().await.unwrap();
        assert_eq!(metrics.total_queries, 3);
        assert_eq!(metrics.successful_queries, 2);
        assert_eq!(metrics.failed_queries, 1);
    }
}
