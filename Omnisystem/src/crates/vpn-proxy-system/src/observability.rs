//! Observability (Metrics, Tracing, Logging)

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct VpnMetrics {
    pub packets_sent: Arc<AtomicU64>,
    pub packets_received: Arc<AtomicU64>,
    pub bytes_sent: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,
    pub errors: Arc<AtomicU64>,
}

impl VpnMetrics {
    pub fn new() -> Self {
        Self {
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_sent(&self, bytes: u64) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_received(&self, bytes: u64) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> VpnStats {
        VpnStats {
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

pub struct VpnStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub errors: u64,
}

impl Default for VpnMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics() {
        let metrics = VpnMetrics::new();
        metrics.record_sent(1024);
        metrics.record_received(2048);
        
        let stats = metrics.get_stats();
        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.bytes_received, 2048);
    }
}
