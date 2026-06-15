//! Observability

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct CrmMetrics {
    pub customers: Arc<AtomicU64>,
    pub events: Arc<AtomicU64>,
    pub decisions: Arc<AtomicU64>,
}

impl CrmMetrics {
    pub fn new() -> Self {
        Self {
            customers: Arc::new(AtomicU64::new(0)),
            events: Arc::new(AtomicU64::new(0)),
            decisions: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_customer(&self) {
        self.customers.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_event(&self) {
        self.events.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_decision(&self) {
        self.decisions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> CrmStats {
        CrmStats {
            customers: self.customers.load(Ordering::Relaxed),
            events: self.events.load(Ordering::Relaxed),
            decisions: self.decisions.load(Ordering::Relaxed),
        }
    }
}

pub struct CrmStats {
    pub customers: u64,
    pub events: u64,
    pub decisions: u64,
}

impl Default for CrmMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics() {
        let m = CrmMetrics::new();
        m.record_customer();
        m.record_event();
        let stats = m.get_stats();
        assert_eq!(stats.customers, 1);
        assert_eq!(stats.events, 1);
    }
}
