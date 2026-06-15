/// Deterministic Job Scheduler for UBVM
use crate::spec::TestSpec;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A job to be executed by a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Language to run the test in
    pub lang: String,
    /// Index of the test case in the spec
    pub test_case_index: usize,
    /// Seed for deterministic execution (optional)
    pub seed: Option<u64>,
}

/// Builds a deterministic schedule of jobs from a test spec
pub struct Scheduler {
    jobs: VecDeque<Job>,
}

impl Scheduler {
    /// Create a new scheduler from a spec
    pub fn new(spec: &TestSpec) -> Self {
        let mut jobs = VecDeque::new();

        // Generate one job per (language, test_case) pair
        for (test_idx, test_case) in spec.test_cases.iter().enumerate() {
            for lang in &spec.languages {
                jobs.push_back(Job {
                    lang: lang.clone(),
                    test_case_index: test_idx,
                    seed: test_case.seed,
                });
            }
        }

        Self { jobs }
    }

    /// Get the next job (deterministic order)
    pub fn next_job(&mut self) -> Option<Job> {
        self.jobs.pop_front()
    }

    /// Get total number of jobs
    pub fn total_jobs(&self) -> usize {
        self.jobs.len()
    }

    /// Peek at the next job without removing it
    pub fn peek_next(&self) -> Option<&Job> {
        self.jobs.front()
    }

    /// Skip n jobs (useful for resuming from checkpoint)
    pub fn skip(&mut self, n: usize) {
        for _ in 0..n {
            self.jobs.pop_front();
        }
    }

    /// Get remaining jobs as a vector (for parallel processing)
    pub fn remaining_jobs(&mut self) -> Vec<Job> {
        self.jobs.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::TestCase;

    fn make_test_spec() -> TestSpec {
        let mut spec = TestSpec::default();
        spec.name = "Test".to_string();
        spec.canonical_source = "fn test() {}".to_string();
        spec.languages = vec!["rust".to_string(), "python".to_string()];
        spec.test_cases = vec![
            TestCase {
                name: "case1".to_string(),
                input: "1".to_string(),
                expected: "2".to_string(),
                seed: Some(42),
            },
            TestCase {
                name: "case2".to_string(),
                input: "2".to_string(),
                expected: "4".to_string(),
                seed: None,
            },
        ];
        spec
    }

    #[test]
    fn test_scheduler_creation() {
        let spec = make_test_spec();
        let scheduler = Scheduler::new(&spec);
        // 2 test cases × 2 languages = 4 jobs
        assert_eq!(scheduler.total_jobs(), 4);
    }

    #[test]
    fn test_scheduler_order() {
        let spec = make_test_spec();
        let mut scheduler = Scheduler::new(&spec);

        let job1 = scheduler.next_job().unwrap();
        assert_eq!(job1.test_case_index, 0);
        assert_eq!(job1.lang, "rust");

        let job2 = scheduler.next_job().unwrap();
        assert_eq!(job2.test_case_index, 0);
        assert_eq!(job2.lang, "python");

        let job3 = scheduler.next_job().unwrap();
        assert_eq!(job3.test_case_index, 1);
        assert_eq!(job3.lang, "rust");
    }

    #[test]
    fn test_scheduler_skip() {
        let spec = make_test_spec();
        let mut scheduler = Scheduler::new(&spec);
        scheduler.skip(2);
        assert_eq!(scheduler.total_jobs(), 2);
    }
}
