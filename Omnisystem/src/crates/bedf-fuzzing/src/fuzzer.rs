use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageGuidedFuzzer {
    pub config: super::FuzzerConfig,
    pub coverage: u32,
    pub edges_seen: HashSet<u64>,
    pub crashes_found: u32,
}

impl CoverageGuidedFuzzer {
    pub fn new(config: super::FuzzerConfig) -> Self {
        Self {
            config,
            coverage: 0,
            edges_seen: HashSet::new(),
            crashes_found: 0,
        }
    }

    pub fn update_coverage(&mut self) {
        self.coverage = self.edges_seen.len() as u32;
    }

    pub fn record_edge(&mut self, edge_id: u64) {
        self.edges_seen.insert(edge_id);
    }

    pub fn record_crash(&mut self) {
        self.crashes_found += 1;
    }

    pub fn coverage_percent(&self) -> f64 {
        (self.coverage as f64 / self.config.max_coverage as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzer_creation() {
        let config = super::super::FuzzerConfig::default();
        let fuzzer = CoverageGuidedFuzzer::new(config);
        assert_eq!(fuzzer.coverage, 0);
        assert_eq!(fuzzer.crashes_found, 0);
    }

    #[test]
    fn test_record_edge() {
        let config = super::super::FuzzerConfig::default();
        let mut fuzzer = CoverageGuidedFuzzer::new(config);
        fuzzer.record_edge(1);
        fuzzer.record_edge(2);
        fuzzer.record_edge(1); // duplicate
        assert_eq!(fuzzer.edges_seen.len(), 2);
    }

    #[test]
    fn test_update_coverage() {
        let config = super::super::FuzzerConfig::default();
        let mut fuzzer = CoverageGuidedFuzzer::new(config);
        fuzzer.record_edge(1);
        fuzzer.record_edge(2);
        fuzzer.update_coverage();
        assert_eq!(fuzzer.coverage, 2);
    }

    #[test]
    fn test_coverage_percent() {
        let mut config = super::super::FuzzerConfig::default();
        config.max_coverage = 100;
        let mut fuzzer = CoverageGuidedFuzzer::new(config);
        fuzzer.edges_seen.insert(1);
        fuzzer.edges_seen.insert(2);
        fuzzer.update_coverage();
        assert_eq!(fuzzer.coverage_percent(), 2.0);
    }
}
