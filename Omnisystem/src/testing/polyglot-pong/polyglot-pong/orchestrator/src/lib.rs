//! Polyglot Pong Orchestrator Library
//!
//! Central coordinator implementing SovereignService for job distribution
//! and result aggregation across all language sandboxes.

pub mod batch_queue;
pub mod language_runner;
// TODO: Update scheduler and comparison modules for new API
// pub mod scheduler;
// mod comparison;

use polyglot_pong_common::*;
// use ai_fallback::{SovereignService, Arbiter, ArbiterConfig, AdvisoryOutput};
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Central Polyglot Pong orchestrator
pub struct Orchestrator {
    pub languages: Vec<Language>,
    pub ai_enabled: bool,
    pub fuzz_enabled: bool,
    pub results: Arc<RwLock<Vec<TestResult>>>,
    pub metrics: Arc<RwLock<AggregateMetrics>>,
    pub batch_queue: Arc<RwLock<Option<batch_queue::BatchQueue>>>,
    pub runners: Arc<tokio::sync::RwLock<std::collections::HashMap<String, language_runner::LanguageRunner>>>,
    pub work_dir: PathBuf,
}

impl Orchestrator {
    /// Create a new orchestrator
    pub async fn new(
        languages: Vec<Language>,
        ai_enabled: bool,
        fuzz_enabled: bool,
    ) -> anyhow::Result<Self> {
        Self::new_with_work_dir(languages, ai_enabled, fuzz_enabled, PathBuf::from(".polyglot-pong")).await
    }

    /// Create orchestrator with custom work directory
    pub async fn new_with_work_dir(
        languages: Vec<Language>,
        ai_enabled: bool,
        fuzz_enabled: bool,
        work_dir: PathBuf,
    ) -> anyhow::Result<Self> {
        // Initialize batch queue and runners
        let queue = batch_queue::BatchQueue::new(work_dir.join("jobs")).await?;
        let mut runners = std::collections::HashMap::new();
        for lang in &languages {
            let runner_path = work_dir.join(format!("runners/{}/runner.py", lang.to_lowercase()));
            runners.insert(
                lang.clone(),
                language_runner::LanguageRunner::new(lang.clone(), runner_path),
            );
        }

        info!("Orchestrator created for {} languages", languages.len());

        Ok(Self {
            languages,
            ai_enabled,
            fuzz_enabled,
            results: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(AggregateMetrics {
                total_tests: 0,
                successful_tests: 0,
                avg_fidelity: 0.0,
                avg_exec_time_us: 0,
                avg_energy_joules: 0.0,
                highest_energy_lang: (String::new(), 0.0),
                lowest_energy_lang: (String::new(), 0.0),
                bugs_discovered: 0,
            })),
            batch_queue: Arc::new(RwLock::new(Some(queue))),
            runners: Arc::new(tokio::sync::RwLock::new(runners)),
            work_dir,
        })
    }

    /// Run the complete test matrix using legacy scheduler (DEPRECATED - use run_batch instead)
    pub async fn run(&self) -> anyhow::Result<()> {
        warn!("run() is deprecated, use run_batch() instead");
        Ok(())
    }

    /// Run using batch processing with real language runners
    pub async fn run_batch(&self, batch_size: usize, frames: usize) -> anyhow::Result<()> {
        let matrix_size = self.languages.len();
        let total_jobs = matrix_size * matrix_size;

        info!("Starting batch test matrix: {} jobs ({}x{})", total_jobs, matrix_size, matrix_size);

        let queue_opt = self.batch_queue.read().await;
        let queue = match queue_opt.as_ref() {
            Some(q) => q,
            None => anyhow::bail!("Batch queue not initialized"),
        };

        // Create all jobs
        queue.create_matrix(&self.languages, 42).await?;
        drop(queue_opt);

        // Get reference trace
        let reference = language_runner::canonical_trace(42, frames);

        // Process in batches
        let mut batch_num = 1;
        let mut total_time = 0u64;

        loop {
            let queue_opt = self.batch_queue.read().await;
            let queue = queue_opt.as_ref().unwrap();
            let batch = queue.fetch_batch(batch_size).await;
            drop(queue_opt);

            if batch.is_empty() {
                break;
            }

            info!("Processing batch {} ({} jobs)", batch_num, batch.len());

            for job in batch {
                let queue_opt = self.batch_queue.read().await;
                queue_opt.as_ref().unwrap().mark_running(&job.id).await?;
                drop(queue_opt);

                let start = std::time::Instant::now();
                match self.execute_batch_job(&job, &reference, frames).await {
                    Ok((fidelity, exec_time_ms)) => {
                        total_time += exec_time_ms;
                        info!(
                            "✓ {} → {} : fidelity = {:.4} ({}ms)",
                            job.src_lang, job.tgt_lang, fidelity, exec_time_ms
                        );

                        let queue_opt = self.batch_queue.read().await;
                        queue_opt.as_ref().unwrap().mark_completed(&job.id, fidelity, exec_time_ms).await?;
                        drop(queue_opt);
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        warn!("✗ {} → {} : {}", job.src_lang, job.tgt_lang, error_msg);

                        let queue_opt = self.batch_queue.read().await;
                        queue_opt.as_ref().unwrap().mark_failed(&job.id, error_msg).await?;
                        drop(queue_opt);
                    }
                }
            }

            let queue_opt = self.batch_queue.read().await;
            let stats = queue_opt.as_ref().unwrap().stats();
            drop(queue_opt);

            info!(
                "Batch {} completed. Progress: {} completed, {} failed. Avg fidelity: {:.4}",
                batch_num, stats.completed, stats.failed, stats.avg_fidelity
            );

            batch_num += 1;
        }

        // Final report
        let queue_opt = self.batch_queue.read().await;
        let stats = queue_opt.as_ref().unwrap().stats();
        drop(queue_opt);

        self.print_batch_report(&stats, total_time);
        Ok(())
    }

    async fn execute_batch_job(
        &self,
        job: &batch_queue::BatchJob,
        reference: &[GameState],
        frames: usize,
    ) -> anyhow::Result<(f32, u64)> {
        let runners = self.runners.read().await;
        if let Some(runner) = runners.get(&job.src_lang) {
            let result = runner.execute(job.seed, frames).await?;

            if !result.success {
                return Err(anyhow::anyhow!(
                    "{}",
                    result.error.unwrap_or_default()
                ));
            }

            let fidelity = language_runner::compute_fidelity(&result.trace, reference);
            Ok((fidelity, result.exec_time_ms))
        } else {
            Err(anyhow::anyhow!("No runner for language: {}", job.src_lang))
        }
    }

    fn print_batch_report(&self, stats: &batch_queue::QueueStats, total_time: u64) {
        let pass_rate = if stats.total > 0 {
            (stats.completed as f32 / stats.total as f32) * 100.0
        } else {
            0.0
        };

        println!("\n════════════════════════════════════════════════════════════════");
        println!("  POLYGLOT PONG BATCH TEST MATRIX RESULTS");
        println!("════════════════════════════════════════════════════════════════");
        println!("  Total Tests:        {}", stats.total);
        println!("  Completed:          {} ({:.1}%)", stats.completed, pass_rate);
        println!("  Failed:             {} ({:.1}%)", stats.failed, (stats.failed as f32 / stats.total as f32) * 100.0);
        println!("  Average Fidelity:   {:.4}", stats.avg_fidelity);
        println!("  Total Time:         {:.1}s", total_time as f32 / 1000.0);
        println!("════════════════════════════════════════════════════════════════\n");
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let langs = vec!["Rust".into(), "Python".into()];
        let orch = Orchestrator::new(langs, false, true).await.unwrap();
        assert_eq!(orch.languages.len(), 2);
    }
}
