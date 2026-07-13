use crate::{AdaptationStrategy, ProtocolError, ProtocolMetrics, ProtocolResult, ProtocolSelection, ProtocolType};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ProtocolSelector {
    metrics: Arc<DashMap<ProtocolType, ProtocolMetrics>>,
    selections: Arc<DashMap<String, ProtocolSelection>>,
}

impl ProtocolSelector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
            selections: Arc::new(DashMap::new()),
        }
    }

    pub async fn record_metrics(&self, metrics: &ProtocolMetrics) -> ProtocolResult<()> {
        self.metrics.insert(metrics.protocol, metrics.clone());
        Ok(())
    }

    pub async fn select_protocol(
        &self,
        selection_id: &str,
        current_protocol: ProtocolType,
        strategy: AdaptationStrategy,
    ) -> ProtocolResult<ProtocolType> {
        let candidates = vec![
            ProtocolType::Http1,
            ProtocolType::Http2,
            ProtocolType::Http3,
            ProtocolType::Grpc,
            ProtocolType::WebSocket,
            ProtocolType::Mqtt,
        ];

        let selected = match strategy {
            AdaptationStrategy::LatencyOptimized => self.select_lowest_latency(&candidates)?,
            AdaptationStrategy::ThroughputOptimized => self.select_highest_throughput(&candidates)?,
            AdaptationStrategy::ReliabilityOptimized => self.select_highest_reliability(&candidates)?,
            AdaptationStrategy::CostOptimized => self.select_lowest_cost(&candidates)?,
        };

        let selection = ProtocolSelection {
            current_protocol,
            candidates: candidates.clone(),
            selected_protocol: selected,
            adaptation_reason: format!("Selected based on {:?} strategy", strategy),
            timestamp: Utc::now(),
        };

        self.selections.insert(selection_id.to_string(), selection);
        Ok(selected)
    }

    fn select_lowest_latency(&self, candidates: &[ProtocolType]) -> ProtocolResult<ProtocolType> {
        candidates
            .iter()
            .filter_map(|proto| self.metrics.get(proto).map(|m| (*proto, m.avg_latency_ms)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(proto, _)| proto)
            .ok_or(ProtocolError::SelectionFailed)
    }

    fn select_highest_throughput(&self, candidates: &[ProtocolType]) -> ProtocolResult<ProtocolType> {
        candidates
            .iter()
            .filter_map(|proto| self.metrics.get(proto).map(|m| (*proto, m.throughput_rps)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(proto, _)| proto)
            .ok_or(ProtocolError::SelectionFailed)
    }

    fn select_highest_reliability(&self, candidates: &[ProtocolType]) -> ProtocolResult<ProtocolType> {
        candidates
            .iter()
            .filter_map(|proto| self.metrics.get(proto).map(|m| (*proto, m.success_rate)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(proto, _)| proto)
            .ok_or(ProtocolError::SelectionFailed)
    }

    fn select_lowest_cost(&self, candidates: &[ProtocolType]) -> ProtocolResult<ProtocolType> {
        candidates
            .iter()
            .filter_map(|proto| {
                self.metrics.get(proto).map(|m| {
                    let cost = m.error_count as f64 + (m.avg_latency_ms * 0.1);
                    (*proto, cost)
                })
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(proto, _)| proto)
            .ok_or(ProtocolError::SelectionFailed)
    }

    pub async fn get_selection(&self, selection_id: &str) -> ProtocolResult<ProtocolSelection> {
        self.selections
            .get(selection_id)
            .map(|entry| entry.clone())
            .ok_or(ProtocolError::ProtocolNotFound)
    }

    pub fn selection_count(&self) -> usize {
        self.selections.len()
    }
}

impl Default for ProtocolSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_metrics() {
        let selector = ProtocolSelector::new();
        let metrics = ProtocolMetrics {
            protocol: ProtocolType::Http2,
            avg_latency_ms: 50.0,
            p99_latency_ms: 100,
            throughput_rps: 1000.0,
            success_rate: 0.99,
            error_count: 1,
        };

        selector.record_metrics(&metrics).await.unwrap();
        assert_eq!(selector.selection_count(), 0);
    }

    #[tokio::test]
    async fn test_select_lowest_latency() {
        let selector = ProtocolSelector::new();

        selector
            .record_metrics(&ProtocolMetrics {
                protocol: ProtocolType::Http1,
                avg_latency_ms: 100.0,
                p99_latency_ms: 200,
                throughput_rps: 500.0,
                success_rate: 0.95,
                error_count: 5,
            })
            .await
            .unwrap();

        selector
            .record_metrics(&ProtocolMetrics {
                protocol: ProtocolType::Http2,
                avg_latency_ms: 50.0,
                p99_latency_ms: 100,
                throughput_rps: 1000.0,
                success_rate: 0.99,
                error_count: 1,
            })
            .await
            .unwrap();

        let selected = selector
            .select_protocol("sel-1", ProtocolType::Http1, AdaptationStrategy::LatencyOptimized)
            .await
            .unwrap();

        assert_eq!(selected, ProtocolType::Http2);
    }

    #[tokio::test]
    async fn test_select_highest_throughput() {
        let selector = ProtocolSelector::new();

        selector
            .record_metrics(&ProtocolMetrics {
                protocol: ProtocolType::Http1,
                avg_latency_ms: 100.0,
                p99_latency_ms: 200,
                throughput_rps: 500.0,
                success_rate: 0.95,
                error_count: 5,
            })
            .await
            .unwrap();

        selector
            .record_metrics(&ProtocolMetrics {
                protocol: ProtocolType::Http2,
                avg_latency_ms: 50.0,
                p99_latency_ms: 100,
                throughput_rps: 1000.0,
                success_rate: 0.99,
                error_count: 1,
            })
            .await
            .unwrap();

        let selected = selector
            .select_protocol("sel-1", ProtocolType::Http1, AdaptationStrategy::ThroughputOptimized)
            .await
            .unwrap();

        assert_eq!(selected, ProtocolType::Http2);
    }

    #[tokio::test]
    async fn test_select_highest_reliability() {
        let selector = ProtocolSelector::new();

        selector
            .record_metrics(&ProtocolMetrics {
                protocol: ProtocolType::Http1,
                avg_latency_ms: 100.0,
                p99_latency_ms: 200,
                throughput_rps: 500.0,
                success_rate: 0.95,
                error_count: 5,
            })
            .await
            .unwrap();

        selector
            .record_metrics(&ProtocolMetrics {
                protocol: ProtocolType::Http2,
                avg_latency_ms: 50.0,
                p99_latency_ms: 100,
                throughput_rps: 1000.0,
                success_rate: 0.99,
                error_count: 1,
            })
            .await
            .unwrap();

        let selected = selector
            .select_protocol("sel-1", ProtocolType::Http1, AdaptationStrategy::ReliabilityOptimized)
            .await
            .unwrap();

        assert_eq!(selected, ProtocolType::Http2);
    }

    #[tokio::test]
    async fn test_get_selection() {
        let selector = ProtocolSelector::new();

        selector
            .record_metrics(&ProtocolMetrics {
                protocol: ProtocolType::Http2,
                avg_latency_ms: 50.0,
                p99_latency_ms: 100,
                throughput_rps: 1000.0,
                success_rate: 0.99,
                error_count: 1,
            })
            .await
            .unwrap();

        selector
            .select_protocol("sel-1", ProtocolType::Http1, AdaptationStrategy::LatencyOptimized)
            .await
            .unwrap();

        let selection = selector.get_selection("sel-1").await.unwrap();
        assert_eq!(selection.selected_protocol, ProtocolType::Http2);
    }

    #[tokio::test]
    async fn test_selection_not_found() {
        let selector = ProtocolSelector::new();
        let result = selector.get_selection("nonexistent").await;

        assert!(result.is_err());
    }
}
