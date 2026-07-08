//! Job scheduling - deterministic core and heuristic variants.

use polyglot_pong_common::*;
use std::collections::VecDeque;
use tracing::debug;

/// Job scheduler - generates and distributes test jobs.
pub struct JobScheduler {
    pub languages: Vec<Language>,
    pub pending: VecDeque<Job>,
    pub spec: spec::CanonicalSpec,
    pub completed: usize,
    pub current_round: u32,
}

impl JobScheduler {
    /// Create a new scheduler for a list of languages.
    pub fn new(languages: Vec<Language>) -> Self {
        let spec = spec::CanonicalSpec::standard();
        let mut pending = VecDeque::new();

        // Generate all (src, tgt) pairs for full conversion matrix
        for (src_idx, src) in languages.iter().enumerate() {
            for (tgt_idx, tgt) in languages.iter().enumerate() {
                let job_id = TestId(uuid::Uuid::new_v4());
                pending.push_back(Job {
                    job_id,
                    source_lang: src.clone(),
                    target_lang: tgt.clone(),
                    conversion_round: 1,
                    canonical_spec: spec.clone(),
                    random_seed: 42 + (src_idx * languages.len() + tgt_idx) as u64,
                });
            }
        }

        debug!(
            "Created job scheduler with {} languages ({} jobs)",
            languages.len(),
            pending.len()
        );

        Self {
            languages,
            pending,
            spec,
            completed: 0,
            current_round: 1,
        }
    }

    /// Deterministic core: round-robin pop (no AI, pure FIFO).
    pub fn next_job_deterministic(&mut self) -> anyhow::Result<Job> {
        self.pending
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("No more jobs in queue"))
    }

    /// Heuristic: prioritize C-like languages (faster to implement).
    pub fn next_job_by_family(&mut self) -> anyhow::Result<Job> {
        let c_like = vec!["Rust", "C", "C++", "Go", "Zig", "Swift", "Kotlin"];
        let functional = vec!["Haskell", "Lisp", "Scala", "OCaml", "Clojure"];

        // First, try to find a job with C-like source language
        for (i, job) in self.pending.iter().enumerate() {
            if c_like.contains(&job.source_lang.as_str()) {
                if let Some(job) = self.pending.remove(i) {
                    return Ok(job);
                }
            }
        }

        // Then, try functional languages
        for (i, job) in self.pending.iter().enumerate() {
            if functional.contains(&job.source_lang.as_str()) {
                if let Some(job) = self.pending.remove(i) {
                    return Ok(job);
                }
            }
        }

        // Fallback: return next deterministic
        self.next_job_deterministic()
    }

    /// Heuristic: prioritize target languages (by implementation difficulty).
    pub fn next_job_by_target_difficulty(&mut self) -> anyhow::Result<Job> {
        let easy_targets = vec!["Python", "JavaScript", "Ruby", "Lua"];

        for (i, job) in self.pending.iter().enumerate() {
            if easy_targets.contains(&job.target_lang.as_str()) {
                if let Some(job) = self.pending.remove(i) {
                    return Ok(job);
                }
            }
        }

        self.next_job_deterministic()
    }

    /// Safe stub: same as deterministic (always returns a job or error).
    pub fn next_job_fallback(&mut self) -> anyhow::Result<Job> {
        self.next_job_deterministic()
    }

    /// Public interface: get next job (used by main loop).
    pub fn next_job(&mut self) -> Option<Job> {
        self.pending.pop_front()
    }

    /// Get scheduler statistics.
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            total_jobs: self.languages.len() * self.languages.len(),
            completed: self.completed,
            remaining: self.pending.len(),
            current_round: self.current_round,
        }
    }

    /// Mark a job as completed.
    pub fn mark_completed(&mut self) {
        self.completed += 1;
    }

    /// Reset for next round (multi-pass testing).
    pub fn next_round(&mut self) {
        self.current_round += 1;
        self.completed = 0;
        // Regenerate pending with next round number
        self.pending.clear();
        for (src_idx, src) in self.languages.iter().enumerate() {
            for (tgt_idx, tgt) in self.languages.iter().enumerate() {
                let job_id = TestId(uuid::Uuid::new_v4());
                self.pending.push_back(Job {
                    job_id,
                    source_lang: src.clone(),
                    target_lang: tgt.clone(),
                    conversion_round: self.current_round,
                    canonical_spec: self.spec.clone(),
                    random_seed: 42
                        + (src_idx * self.languages.len() + tgt_idx) as u64
                        + (self.current_round as u64 * 1000),
                });
            }
        }
    }
}

/// Scheduler statistics.
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total_jobs: usize,
    pub completed: usize,
    pub remaining: usize,
    pub current_round: u32,
}

impl SchedulerStats {
    /// Compute progress as percentage.
    pub fn progress_percent(&self) -> f32 {
        if self.total_jobs == 0 {
            100.0
        } else {
            (self.completed as f32 / self.total_jobs as f32) * 100.0
        }
    }

    /// Estimate time remaining (rough).
    pub fn estimated_remaining_seconds(&self, avg_seconds_per_job: f64) -> f64 {
        self.remaining as f64 * avg_seconds_per_job
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let langs = vec!["Rust".into(), "Python".into(), "Go".into()];
        let scheduler = JobScheduler::new(langs.clone());

        assert_eq!(scheduler.languages.len(), 3);
        assert_eq!(scheduler.pending.len(), 9); // 3 x 3 matrix
    }

    #[test]
    fn test_scheduler_deterministic() {
        let langs = vec!["Rust".into(), "Python".into()];
        let mut scheduler = JobScheduler::new(langs);

        let job1 = scheduler.next_job_deterministic().unwrap();
        let job2 = scheduler.next_job_deterministic().unwrap();

        assert_ne!(job1.job_id, job2.job_id);
        assert_eq!(scheduler.pending.len(), 2);
    }

    #[test]
    fn test_scheduler_stats() {
        let langs = vec!["A".into(), "B".into()];
        let mut scheduler = JobScheduler::new(langs);

        let stats = scheduler.stats();
        assert_eq!(stats.total_jobs, 4);
        assert_eq!(stats.remaining, 4);
        assert_eq!(stats.completed, 0);
        assert_eq!(stats.progress_percent(), 0.0);

        scheduler.mark_completed();
        scheduler.mark_completed();

        let stats = scheduler.stats();
        assert_eq!(stats.completed, 2);
        assert_eq!(stats.progress_percent(), 50.0);
    }

    #[test]
    fn test_scheduler_next_round() {
        let langs = vec!["Rust".into(), "Python".into()];
        let mut scheduler = JobScheduler::new(langs);

        assert_eq!(scheduler.current_round, 1);

        // Consume all jobs
        while scheduler.next_job().is_some() {}

        scheduler.next_round();
        assert_eq!(scheduler.current_round, 2);
        assert_eq!(scheduler.pending.len(), 4); // Regenerated
    }
}
