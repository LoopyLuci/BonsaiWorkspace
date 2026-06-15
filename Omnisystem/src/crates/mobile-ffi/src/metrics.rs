//! Performance metrics collection and reporting

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

/// Frame-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetrics {
    /// Timestamp when frame was received (microseconds since epoch)
    pub input_timestamp_us: i64,
    /// Timestamp when frame was decoded (microseconds since epoch)
    pub output_timestamp_us: i64,
    /// Decode latency in microseconds
    pub latency_us: i64,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Frame size in bytes
    pub size_bytes: u32,
    /// Presentation timestamp from stream (microseconds)
    pub pts_us: i64,
}

impl FrameMetrics {
    /// Create new frame metrics
    pub fn new(width: u32, height: u32, size_bytes: u32, pts_us: i64) -> Self {
        let now = current_time_us();
        FrameMetrics {
            input_timestamp_us: now,
            output_timestamp_us: now,
            latency_us: 0,
            width,
            height,
            size_bytes,
            pts_us,
        }
    }

    /// Mark frame as decoded and calculate latency
    pub fn mark_decoded(&mut self) {
        self.output_timestamp_us = current_time_us();
        self.latency_us = self.output_timestamp_us - self.input_timestamp_us;
    }
}

/// Aggregated decoder metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoderMetrics {
    /// Average decode latency in microseconds
    pub avg_decode_latency_us: i64,
    /// Maximum decode latency in microseconds
    pub max_decode_latency_us: i64,
    /// Total frames successfully decoded
    pub frames_decoded: u64,
    /// Total frames dropped
    pub frames_dropped: u64,
    /// Last frame presentation timestamp (microseconds)
    pub last_timestamp_us: Option<i64>,
    /// Last frame width
    pub last_width: Option<u32>,
    /// Last frame height
    pub last_height: Option<u32>,
    /// Total bytes decoded
    pub total_bytes: u64,
    /// Minimum decode latency in microseconds
    pub min_decode_latency_us: i64,
    /// Decoder startup timestamp
    pub start_time_us: i64,
    /// Elapsed time since startup (microseconds)
    pub elapsed_time_us: i64,
}

impl DecoderMetrics {
    /// Calculate frames per second
    pub fn fps(&self) -> f64 {
        if self.elapsed_time_us == 0 {
            return 0.0;
        }
        (self.frames_decoded as f64 * 1_000_000.0) / self.elapsed_time_us as f64
    }

    /// Calculate average throughput in megabytes per second
    pub fn throughput_mbps(&self) -> f64 {
        if self.elapsed_time_us == 0 {
            return 0.0;
        }
        (self.total_bytes as f64 * 8.0) / (self.elapsed_time_us as f64 / 1_000_000.0) / 1_000_000.0
    }

    /// Calculate drop rate as percentage
    pub fn drop_rate_percent(&self) -> f64 {
        let total = self.frames_decoded + self.frames_dropped;
        if total == 0 {
            return 0.0;
        }
        (self.frames_dropped as f64 / total as f64) * 100.0
    }
}

impl Default for DecoderMetrics {
    fn default() -> Self {
        let now = current_time_us();
        DecoderMetrics {
            avg_decode_latency_us: 0,
            max_decode_latency_us: 0,
            frames_decoded: 0,
            frames_dropped: 0,
            last_timestamp_us: None,
            last_width: None,
            last_height: None,
            total_bytes: 0,
            min_decode_latency_us: i64::MAX,
            start_time_us: now,
            elapsed_time_us: 0,
        }
    }
}

/// Thread-safe metrics collector
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    frames_decoded: Arc<AtomicU64>,
    frames_dropped: Arc<AtomicU64>,
    total_latency_us: Arc<AtomicI64>,
    max_latency_us: Arc<AtomicI64>,
    min_latency_us: Arc<AtomicI64>,
    total_bytes: Arc<AtomicU64>,
    last_timestamp_us: Arc<AtomicI64>,
    last_width: Arc<AtomicU64>,
    last_height: Arc<AtomicU64>,
    start_time_us: i64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        MetricsCollector {
            frames_decoded: Arc::new(AtomicU64::new(0)),
            frames_dropped: Arc::new(AtomicU64::new(0)),
            total_latency_us: Arc::new(AtomicI64::new(0)),
            max_latency_us: Arc::new(AtomicI64::new(0)),
            min_latency_us: Arc::new(AtomicI64::new(i64::MAX)),
            total_bytes: Arc::new(AtomicU64::new(0)),
            last_timestamp_us: Arc::new(AtomicI64::new(-1)),
            last_width: Arc::new(AtomicU64::new(0)),
            last_height: Arc::new(AtomicU64::new(0)),
            start_time_us: current_time_us(),
        }
    }

    /// Record a successfully decoded frame
    pub fn record_frame_decoded(&self, latency_us: i64, width: u32, height: u32, size: u64, timestamp_us: i64) {
        self.frames_decoded.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(latency_us, Ordering::Relaxed);

        // Update max latency
        let mut max = self.max_latency_us.load(Ordering::Relaxed);
        while latency_us > max {
            match self.max_latency_us.compare_exchange_weak(
                max,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => max = actual,
            }
        }

        // Update min latency
        let mut min = self.min_latency_us.load(Ordering::Relaxed);
        while latency_us < min {
            match self.min_latency_us.compare_exchange_weak(
                min,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => min = actual,
            }
        }

        self.total_bytes.fetch_add(size, Ordering::Relaxed);
        self.last_timestamp_us.store(timestamp_us, Ordering::Relaxed);
        self.last_width.store(width as u64, Ordering::Relaxed);
        self.last_height.store(height as u64, Ordering::Relaxed);
    }

    /// Record a dropped frame
    pub fn record_frame_dropped(&self) {
        self.frames_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> DecoderMetrics {
        let frames_decoded = self.frames_decoded.load(Ordering::Relaxed);
        let frames_dropped = self.frames_dropped.load(Ordering::Relaxed);
        let total_latency_us = self.total_latency_us.load(Ordering::Relaxed);
        let max_latency_us = self.max_latency_us.load(Ordering::Relaxed);
        let min_latency_us = self.min_latency_us.load(Ordering::Relaxed);
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);
        let last_timestamp_us = self.last_timestamp_us.load(Ordering::Relaxed);
        let last_width = self.last_width.load(Ordering::Relaxed);
        let last_height = self.last_height.load(Ordering::Relaxed);

        let avg_latency = if frames_decoded > 0 {
            total_latency_us / frames_decoded as i64
        } else {
            0
        };

        let now = current_time_us();
        let elapsed_time_us = now - self.start_time_us;

        DecoderMetrics {
            avg_decode_latency_us: avg_latency,
            max_decode_latency_us: if max_latency_us == 0 { 0 } else { max_latency_us },
            frames_decoded,
            frames_dropped,
            last_timestamp_us: if last_timestamp_us < 0 { None } else { Some(last_timestamp_us) },
            last_width: if last_width == 0 { None } else { Some(last_width as u32) },
            last_height: if last_height == 0 { None } else { Some(last_height as u32) },
            total_bytes,
            min_decode_latency_us: if min_latency_us == i64::MAX { 0 } else { min_latency_us },
            start_time_us: self.start_time_us,
            elapsed_time_us,
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.frames_decoded.store(0, Ordering::Relaxed);
        self.frames_dropped.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
        self.max_latency_us.store(0, Ordering::Relaxed);
        self.min_latency_us.store(i64::MAX, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.last_timestamp_us.store(-1, Ordering::Relaxed);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current time in microseconds since UNIX epoch
pub fn current_time_us() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_metrics_creation() {
        let fm = FrameMetrics::new(1920, 1080, 3_110_400, 33_333);
        assert_eq!(fm.width, 1920);
        assert_eq!(fm.height, 1080);
        assert_eq!(fm.pts_us, 33_333);
    }

    #[test]
    fn test_metrics_collector_basic() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.frames_decoded.load(Ordering::Relaxed), 0);

        collector.record_frame_decoded(5000, 1920, 1080, 3_110_400, 33_333);
        assert_eq!(collector.frames_decoded.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_metrics_snapshot() {
        let collector = MetricsCollector::new();
        collector.record_frame_decoded(8000, 1920, 1080, 3_110_400, 33_333);
        collector.record_frame_decoded(9000, 1920, 1080, 3_110_400, 66_666);
        collector.record_frame_dropped();

        let metrics = collector.snapshot();
        assert_eq!(metrics.frames_decoded, 2);
        assert_eq!(metrics.frames_dropped, 1);
        assert_eq!(metrics.avg_decode_latency_us, 8500);
        assert_eq!(metrics.max_decode_latency_us, 9000);
        assert_eq!(metrics.min_decode_latency_us, 8000);
    }

    #[test]
    fn test_drop_rate_calculation() {
        let collector = MetricsCollector::new();
        collector.record_frame_decoded(5000, 1920, 1080, 3_110_400, 33_333);
        collector.record_frame_dropped();

        let metrics = collector.snapshot();
        let drop_rate = metrics.drop_rate_percent();
        assert!((drop_rate - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_fps_calculation() {
        let collector = MetricsCollector::new();

        // Record 60 frames with 1 second elapsed
        for i in 0..60 {
            collector.record_frame_decoded(5000 + i as i64, 1920, 1080, 3_110_400, (33_333 * i as i64));
        }

        let metrics = collector.snapshot();
        // FPS should be approximately 60
        assert!(metrics.fps() >= 50.0 && metrics.fps() <= 70.0);
    }
}
