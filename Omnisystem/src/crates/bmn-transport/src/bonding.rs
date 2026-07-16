// Multi-path bonding (WiFi + 5G + Ethernet)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkPath {
    WiFi,
    Cellular5G,
    Ethernet,
    Satellite,
}

#[derive(Debug, Clone)]
pub struct PathHealth {
    pub path: NetworkPath,
    pub latency_ms: f32,
    pub jitter_ms: f32,
    pub loss_percent: f32,
    pub bandwidth_mbps: f32,
    pub is_healthy: bool,
}

pub struct MultiPathBonding {
    paths: Vec<PathHealth>,
}

impl MultiPathBonding {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
        }
    }

    pub fn add_path(&mut self, path: PathHealth) {
        self.paths.push(path);
    }

    pub fn select_best_path(&self) -> Option<NetworkPath> {
        // Select path with lowest latency + jitter, highest health
        self.paths
            .iter()
            .filter(|p| p.is_healthy)
            .min_by_key(|p| (p.latency_ms + p.jitter_ms) as i32)
            .map(|p| p.path)
    }

    pub fn failover_paths(&self) -> Vec<NetworkPath> {
        // All healthy paths in priority order
        let mut paths: Vec<_> = self.paths
            .iter()
            .filter(|p| p.is_healthy)
            .collect();
        paths.sort_by_key(|p| (p.latency_ms + p.jitter_ms) as i32);
        paths.iter().map(|p| p.path).collect()
    }
}

impl Default for MultiPathBonding {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_path_bonding() {
        let mut bonding = MultiPathBonding::new();

        bonding.add_path(PathHealth {
            path: NetworkPath::Ethernet,
            latency_ms: 5.0,
            jitter_ms: 1.0,
            loss_percent: 0.0,
            bandwidth_mbps: 1000.0,
            is_healthy: true,
        });

        bonding.add_path(PathHealth {
            path: NetworkPath::WiFi,
            latency_ms: 50.0,
            jitter_ms: 10.0,
            loss_percent: 0.5,
            bandwidth_mbps: 100.0,
            is_healthy: true,
        });

        let best = bonding.select_best_path();
        assert_eq!(best, Some(NetworkPath::Ethernet));
    }
}
