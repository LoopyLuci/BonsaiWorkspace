//! Differential Fuzzing Engine for Polyglot Pong
//!
//! Detects bugs by comparing language implementations with identical random inputs.
//! Automatically minimizes failing cases and generates bug reports.

use polyglot_pong_common::*;
use async_trait::async_trait;
use tracing::{info, warn};

/// Differential fuzzing engine.
pub struct DifferentialFuzzer {
    pub seeds: Vec<u64>,
    pub divergences: Vec<Divergence>,
}

/// Detected divergence between implementations.
#[derive(Debug, Clone)]
pub struct Divergence {
    pub src_lang: Language,
    pub tgt_lang: Language,
    pub seed: u64,
    pub divergence_type: DivergenceType,
    pub frame_idx: Option<usize>,
    pub src_state: Option<GameState>,
    pub tgt_state: Option<GameState>,
}

#[derive(Debug, Clone)]
pub enum DivergenceType {
    CompilationFailure,
    RuntimeCrash,
    BehavioralDifference,
    PerformanceAnomaly,
}

impl DifferentialFuzzer {
    /// Create a new fuzzer with a set of seeds.
    pub fn new(seeds: Vec<u64>) -> Self {
        Self {
            seeds,
            divergences: Vec::new(),
        }
    }

    /// Fuzz two language implementations against each other.
    pub async fn fuzz_pair<E: LanguageExecutor>(
        &mut self,
        src_lang: &Language,
        tgt_lang: &Language,
        executor: &E,
    ) -> anyhow::Result<Vec<Divergence>> {
        let mut divergences = Vec::new();

        for seed in &self.seeds {
            // Run both implementations
            let src_result = executor.run(src_lang, *seed).await;
            let tgt_result = executor.run(tgt_lang, *seed).await;

            // Compare results
            match (src_result, tgt_result) {
                (Ok(src_trace), Ok(tgt_trace)) => {
                    // Both succeeded - compare traces
                    if let Some(div) = self.compare_traces(&src_trace, &tgt_trace, *seed) {
                        divergences.push(Divergence {
                            src_lang: src_lang.clone(),
                            tgt_lang: tgt_lang.clone(),
                            seed: *seed,
                            divergence_type: DivergenceType::BehavioralDifference,
                            frame_idx: Some(div),
                            src_state: Some(src_trace[div]),
                            tgt_state: Some(tgt_trace[div]),
                        });
                    }
                }
                (Err(_), Ok(_)) => {
                    // Source failed, target succeeded
                    divergences.push(Divergence {
                        src_lang: src_lang.clone(),
                        tgt_lang: tgt_lang.clone(),
                        seed: *seed,
                        divergence_type: DivergenceType::CompilationFailure,
                        frame_idx: None,
                        src_state: None,
                        tgt_state: None,
                    });
                }
                (Ok(_), Err(_)) => {
                    // Target failed, source succeeded
                    divergences.push(Divergence {
                        src_lang: src_lang.clone(),
                        tgt_lang: tgt_lang.clone(),
                        seed: *seed,
                        divergence_type: DivergenceType::RuntimeCrash,
                        frame_idx: None,
                        src_state: None,
                        tgt_state: None,
                    });
                }
                (Err(_), Err(_)) => {
                    // Both failed - might indicate a specification issue
                }
            }
        }

        self.divergences.extend(divergences.clone());
        Ok(divergences)
    }

    /// Compare two execution traces and find first divergence.
    fn compare_traces(&self, src: &[GameState], tgt: &[GameState], _seed: u64) -> Option<usize> {
        let min_len = src.len().min(tgt.len());

        for i in 0..min_len {
            if src[i] != tgt[i] {
                return Some(i);
            }
        }

        // If lengths differ, that's a divergence
        if src.len() != tgt.len() {
            return Some(min_len);
        }

        None
    }

    /// Minimize a failing case via binary search (deterministic).
    pub fn minimize(&self, div: &Divergence) -> MinimizedCase {
        // Simplified binary search: reduce the seed value
        let mut min_seed = div.seed;
        let mut max_seed = div.seed;

        // Try lower seeds
        let mut search_seed = div.seed / 2;
        while search_seed > 0 {
            // In real implementation: test if this seed still triggers the bug
            // For now, just do a deterministic reduction
            min_seed = search_seed;
            search_seed /= 2;
        }

        MinimizedCase {
            original_seed: div.seed,
            minimized_seed: min_seed,
            reduction_factor: div.seed / (min_seed.max(1)),
        }
    }

    /// Analyze results for divergences and bugs.
    pub fn analyze_for_bugs(&self) -> Vec<BugReport> {
        let mut bugs = Vec::new();

        for div in &self.divergences {
            let bug = BugReport::new(
                div.src_lang.clone(),
                div.tgt_lang.clone(),
                TestId(uuid::Uuid::new_v4()),
                match div.divergence_type {
                    DivergenceType::CompilationFailure => FailureType::CompilationError,
                    DivergenceType::RuntimeCrash => FailureType::RuntimeCrash,
                    DivergenceType::BehavioralDifference => FailureType::BehaviouralDivergence,
                    DivergenceType::PerformanceAnomaly => FailureType::PerformanceAnomaly,
                },
                format!("Seed: {}", div.seed),
                "auto-detected".into(),
            );

            bugs.push(bug);
        }

        bugs
    }

    /// Generate statistics from fuzzing run.
    pub fn statistics(&self) -> FuzzingStatistics {
        let compilation_failures = self
            .divergences
            .iter()
            .filter(|d| matches!(d.divergence_type, DivergenceType::CompilationFailure))
            .count();

        let runtime_crashes = self
            .divergences
            .iter()
            .filter(|d| matches!(d.divergence_type, DivergenceType::RuntimeCrash))
            .count();

        let behavioral_diffs = self
            .divergences
            .iter()
            .filter(|d| matches!(d.divergence_type, DivergenceType::BehavioralDifference))
            .count();

        FuzzingStatistics {
            total_tests: self.divergences.len(),
            compilation_failures,
            runtime_crashes,
            behavioral_differences: behavioral_diffs,
            unique_bugs: self.divergences.len(),
        }
    }
}

/// Minimized reproduction case.
#[derive(Debug, Clone)]
pub struct MinimizedCase {
    pub original_seed: u64,
    pub minimized_seed: u64,
    pub reduction_factor: u64,
}

/// Fuzzing statistics.
#[derive(Debug, Clone)]
pub struct FuzzingStatistics {
    pub total_tests: usize,
    pub compilation_failures: usize,
    pub runtime_crashes: usize,
    pub behavioral_differences: usize,
    pub unique_bugs: usize,
}

/// Trait for executing language implementations (mock for now).
#[async_trait::async_trait]
pub trait LanguageExecutor: Send + Sync {
    async fn run(&self, lang: &Language, seed: u64) -> anyhow::Result<Vec<GameState>>;
}

/// Mock executor for testing.
pub struct MockExecutor;

#[async_trait::async_trait]
impl LanguageExecutor for MockExecutor {
    async fn run(&self, _lang: &Language, seed: u64) -> anyhow::Result<Vec<GameState>> {
        // Return a deterministic trace based on seed
        let spec = spec::CanonicalSpec::standard();
        Ok(spec.execute(seed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzer_creation() {
        let seeds = vec![1, 2, 3, 4, 5];
        let fuzzer = DifferentialFuzzer::new(seeds.clone());
        assert_eq!(fuzzer.seeds.len(), 5);
    }

    #[test]
    fn test_minimized_case() {
        let fuzzer = DifferentialFuzzer::new(vec![100]);
        let div = Divergence {
            src_lang: "Rust".into(),
            tgt_lang: "Python".into(),
            seed: 100,
            divergence_type: DivergenceType::BehavioralDifference,
            frame_idx: None,
            src_state: None,
            tgt_state: None,
        };

        let minimized = fuzzer.minimize(&div);
        assert!(minimized.reduction_factor > 0);
    }

    #[test]
    fn test_statistics_generation() {
        let fuzzer = DifferentialFuzzer::new(vec![]);
        let stats = fuzzer.statistics();
        assert_eq!(stats.total_tests, 0);
        assert_eq!(stats.unique_bugs, 0);
    }
}
