use crate::routing::BackendInstance;

pub fn select_best(instances: &[BackendInstance]) -> Option<BackendInstance> {
    instances
        .iter()
        .min_by_key(|i| i.load as u64 * 2 + i.latency_ms as u64)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(url: &str, load: u32, latency_ms: u32) -> BackendInstance {
        BackendInstance {
            service: "svc".to_string(),
            url: url.to_string(),
            load,
            latency_ms,
        }
    }

    #[test]
    fn test_selects_lowest_weighted_score() {
        let instances = vec![
            instance("a", 50, 20),
            instance("b", 10, 5),
            instance("c", 20, 8),
        ];

        let best = select_best(&instances).unwrap();
        assert_eq!(best.url, "b"); // score 10*2+5=25 vs 20*2+8=48 vs 50*2+20=120
    }

    #[test]
    fn test_empty_instances_returns_none() {
        assert!(select_best(&[]).is_none());
    }
}
