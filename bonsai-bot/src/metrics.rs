use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct Metrics {
    pub messages_inbound:       AtomicU64,
    pub messages_processed:     AtomicU64,
    pub messages_queued_full:   AtomicU64,
    pub buddy_requests:         AtomicU64,
    pub buddy_errors:           AtomicU64,
    pub buddy_circuit_opens:    AtomicU64,
    pub sanitize_rejected_too_long:       AtomicU64,
    pub sanitize_rejected_protocol:       AtomicU64,
    pub dedup_hits:             AtomicU64,
    pub rate_limit_hits:        AtomicU64,
    pub allowlist_denials:      AtomicU64,
    pub confirms_created:       AtomicU64,
    pub confirms_resolved:      AtomicU64,
    pub confirms_expired:       AtomicU64,
}

impl Metrics {
    pub fn sanitize_rejected(&self, reason: &str) {
        match reason {
            "too_long"          => { self.sanitize_rejected_too_long.fetch_add(1, Ordering::Relaxed); }
            "protocol_boundary" => { self.sanitize_rejected_protocol.fetch_add(1, Ordering::Relaxed); }
            _ => {}
        }
    }

    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "messages_inbound":       self.messages_inbound.load(Ordering::Relaxed),
            "messages_processed":     self.messages_processed.load(Ordering::Relaxed),
            "messages_queued_full":   self.messages_queued_full.load(Ordering::Relaxed),
            "buddy_requests":         self.buddy_requests.load(Ordering::Relaxed),
            "buddy_errors":           self.buddy_errors.load(Ordering::Relaxed),
            "buddy_circuit_opens":    self.buddy_circuit_opens.load(Ordering::Relaxed),
            "sanitize_rejected": {
                "too_long":           self.sanitize_rejected_too_long.load(Ordering::Relaxed),
                "protocol_boundary":  self.sanitize_rejected_protocol.load(Ordering::Relaxed),
            },
            "dedup_hits":             self.dedup_hits.load(Ordering::Relaxed),
            "rate_limit_hits":        self.rate_limit_hits.load(Ordering::Relaxed),
            "allowlist_denials":      self.allowlist_denials.load(Ordering::Relaxed),
            "confirms_created":       self.confirms_created.load(Ordering::Relaxed),
            "confirms_resolved":      self.confirms_resolved.load(Ordering::Relaxed),
            "confirms_expired":       self.confirms_expired.load(Ordering::Relaxed),
        })
    }
}

pub type SharedMetrics = Arc<Metrics>;
