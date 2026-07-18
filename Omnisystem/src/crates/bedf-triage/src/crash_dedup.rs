use blake3;
use std::collections::HashSet;

pub type CrashSignature = String;

pub struct CrashDeduplicator {
    seen_signatures: HashSet<CrashSignature>,
}

impl CrashDeduplicator {
    pub fn new() -> Self {
        Self {
            seen_signatures: HashSet::new(),
        }
    }

    pub fn compute_signature(&self, stack_trace: &str) -> CrashSignature {
        let hash = blake3::hash(stack_trace.as_bytes());
        hash.to_hex().to_string()
    }

    pub fn is_duplicate(&self, signature: &CrashSignature) -> bool {
        self.seen_signatures.contains(signature)
    }

    pub fn record_crash(&mut self, signature: CrashSignature) {
        self.seen_signatures.insert(signature);
    }

    pub fn total_unique_crashes(&self) -> usize {
        self.seen_signatures.len()
    }
}

impl Default for CrashDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedup_creation() {
        let dedup = CrashDeduplicator::new();
        assert_eq!(dedup.total_unique_crashes(), 0);
    }

    #[test]
    fn test_compute_signature() {
        let dedup = CrashDeduplicator::new();
        let sig1 = dedup.compute_signature("panic at index 0");
        let sig2 = dedup.compute_signature("panic at index 0");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_duplicate_detection() {
        let mut dedup = CrashDeduplicator::new();
        let sig = dedup.compute_signature("panic!");
        assert!(!dedup.is_duplicate(&sig));

        dedup.record_crash(sig.clone());
        assert!(dedup.is_duplicate(&sig));
    }

    #[test]
    fn test_unique_crashes() {
        let mut dedup = CrashDeduplicator::new();
        dedup.record_crash(dedup.compute_signature("crash 1"));
        dedup.record_crash(dedup.compute_signature("crash 2"));
        assert_eq!(dedup.total_unique_crashes(), 2);
    }
}
