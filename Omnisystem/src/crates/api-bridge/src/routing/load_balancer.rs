use crate::routing::BackendInstance;

pub fn select_best(instances: &[BackendInstance]) -> Option<BackendInstance> {
    instances
        .iter()
        .min_by_key(|i| i.load as u64 * 2 + i.latency_ms as u64)
        .cloned()
}
