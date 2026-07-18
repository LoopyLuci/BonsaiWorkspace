// Metrics and telemetry for BMN

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Stream health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetrics {
    pub frames_captured: u64,
    pub frames_encoded: u64,
    pub frames_dropped: u64,
    pub frames_sent: u64,
    pub cpu_usage_percent: f32,
    pub gpu_usage_percent: f32,
    pub memory_mb: u64,
    pub bitrate_kbps: f32,
    pub fps: f32,
    pub latency_ms: f32,
    pub viewer_count: u64,
    pub relay_count: u64,
}

impl Default for StreamMetrics {
    fn default() -> Self {
        Self {
            frames_captured: 0,
            frames_encoded: 0,
            frames_dropped: 0,
            frames_sent: 0,
            cpu_usage_percent: 0.0,
            gpu_usage_percent: 0.0,
            memory_mb: 0,
            bitrate_kbps: 0.0,
            fps: 0.0,
            latency_ms: 0.0,
            viewer_count: 0,
            relay_count: 0,
        }
    }
}

/// Centralized metrics collector
pub struct MetricsCollector {
    metrics: Arc<RwLock<StreamMetrics>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(StreamMetrics::default())),
        }
    }

    pub async fn get_snapshot(&self) -> StreamMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn record_frame_captured(&self) {
        let mut m = self.metrics.write().await;
        m.frames_captured += 1;
    }

    pub async fn record_frame_encoded(&self) {
        let mut m = self.metrics.write().await;
        m.frames_encoded += 1;
    }

    pub async fn record_frame_dropped(&self) {
        let mut m = self.metrics.write().await;
        m.frames_dropped += 1;
    }

    pub async fn record_frame_sent(&self) {
        let mut m = self.metrics.write().await;
        m.frames_sent += 1;
    }

    pub async fn update_cpu_usage(&self, usage: f32) {
        let mut m = self.metrics.write().await;
        m.cpu_usage_percent = usage;
    }

    pub async fn update_gpu_usage(&self, usage: f32) {
        let mut m = self.metrics.write().await;
        m.gpu_usage_percent = usage;
    }

    pub async fn update_memory_usage(&self, mb: u64) {
        let mut m = self.metrics.write().await;
        m.memory_mb = mb;
    }

    pub async fn update_bitrate(&self, kbps: f32) {
        let mut m = self.metrics.write().await;
        m.bitrate_kbps = kbps;
    }

    pub async fn update_fps(&self, fps: f32) {
        let mut m = self.metrics.write().await;
        m.fps = fps;
    }

    pub async fn update_latency(&self, ms: f32) {
        let mut m = self.metrics.write().await;
        m.latency_ms = ms;
    }

    pub async fn update_viewer_count(&self, count: u64) {
        let mut m = self.metrics.write().await;
        m.viewer_count = count;
    }

    pub async fn update_relay_count(&self, count: u64) {
        let mut m = self.metrics.write().await;
        m.relay_count = count;
    }

    pub async fn drop_rate(&self) -> f32 {
        let m = self.metrics.read().await;
        if m.frames_captured == 0 {
            return 0.0;
        }
        (m.frames_dropped as f32 / m.frames_captured as f32) * 100.0
    }

    pub async fn encode_rate(&self) -> f32 {
        let m = self.metrics.read().await;
        if m.frames_captured == 0 {
            return 0.0;
        }
        (m.frames_encoded as f32 / m.frames_captured as f32) * 100.0
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.record_frame_captured().await;
        collector.record_frame_captured().await;
        collector.record_frame_encoded().await;
        collector.record_frame_dropped().await;

        let snapshot = collector.get_snapshot().await;
        assert_eq!(snapshot.frames_captured, 2);
        assert_eq!(snapshot.frames_encoded, 1);
        assert_eq!(snapshot.frames_dropped, 1);

        assert_eq!(collector.drop_rate().await, 50.0);
        assert_eq!(collector.encode_rate().await, 50.0);
    }

    #[tokio::test]
    async fn test_resource_updates() {
        let collector = MetricsCollector::new();

        collector.update_cpu_usage(45.5).await;
        collector.update_gpu_usage(60.0).await;
        collector.update_memory_usage(512).await;

        let snapshot = collector.get_snapshot().await;
        assert_eq!(snapshot.cpu_usage_percent, 45.5);
        assert_eq!(snapshot.gpu_usage_percent, 60.0);
        assert_eq!(snapshot.memory_mb, 512);
    }
}
