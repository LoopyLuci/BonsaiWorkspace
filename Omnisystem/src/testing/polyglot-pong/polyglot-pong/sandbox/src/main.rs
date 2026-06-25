//! Polyglot Pong Sandbox - Daemon Process
//!
//! Listens for jobs from the orchestrator via TransferDaemon
//! and executes Pong implementations, returning results.

use clap::Parser;
use polyglot_pong_sandbox::{Sandbox, JobRequest};
use polyglot_pong_common::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(name = "Polyglot Pong Sandbox")]
#[command(about = "Language-specific execution sandbox")]
struct Args {
    /// Language this sandbox is responsible for
    #[arg(long)]
    language: String,

    /// Orchestrator address (IP:port)
    #[arg(long, default_value = "127.0.0.1:9000")]
    orchestrator: String,

    /// Enable AI enhancements
    #[arg(long, default_value = "false")]
    ai: bool,

    /// Max jobs to process before exit (0 = infinite)
    #[arg(long, default_value = "0")]
    max_jobs: u32,

    /// Port to listen on for incoming jobs
    #[arg(long, default_value = "0")]
    listen_port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    info!("Starting Polyglot Pong Sandbox");
    info!("Language: {}", args.language);
    info!("Orchestrator: {}", args.orchestrator);
    info!("AI Enabled: {}", args.ai);

    // Create sandbox
    let sandbox = Arc::new(Sandbox::new(args.language.clone(), args.ai).await?);
    info!("Sandbox initialized for language: {}", args.language);

    // Run daemon loop
    run_daemon(sandbox, &args).await?;

    Ok(())
}

/// Run the main daemon loop
async fn run_daemon(
    sandbox: Arc<Sandbox>,
    args: &Args,
) -> anyhow::Result<()> {
    info!("Sandbox daemon starting on 0.0.0.0:{}", args.listen_port);

    let mut job_count = 0;
    let max_jobs = if args.max_jobs > 0 { args.max_jobs } else { u32::MAX };

    // Main event loop
    loop {
        if job_count >= max_jobs {
            info!("Reached max jobs limit ({}), shutting down", max_jobs);
            break;
        }

        // Wait for job from orchestrator (simulated)
        // In production: listen on socket or use TransferDaemon
        match receive_job().await {
            Ok(job) => {
                job_count += 1;
                info!("Received job #{}: {} -> {}", job_count, job.src_lang, job.tgt_lang);

                // Execute the job
                match sandbox.execute_job(&job.src_lang, job.seed).await {
                    Ok(result) => {
                        info!(
                            "Job #{} completed successfully (fidelity: {:.3})",
                            job_count, result.fidelity
                        );

                        // Send result back to orchestrator
                        if let Err(e) = send_result(&result, &args.orchestrator).await {
                            warn!("Failed to send result: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Job #{} failed: {}", job_count, e);
                    }
                }
            }
            Err(e) => {
                warn!("Error receiving job: {}", e);
                // Sleep before retry
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        // Check shutdown signal periodically
        if should_shutdown().await {
            info!("Shutdown signal received");
            break;
        }
    }

    info!("Sandbox daemon shutting down (processed {} jobs)", job_count);
    Ok(())
}

/// Receive a job from the orchestrator
async fn receive_job() -> anyhow::Result<JobRequest> {
    // In production: receive from TransferDaemon or socket
    // For MVP: simulate receiving jobs
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Return a mock job
    Ok(JobRequest {
        src_lang: "Rust".into(),
        tgt_lang: "Python".into(),
        seed: 42,
    })
}

/// Send result back to orchestrator
async fn send_result(result: &TestResult, _orchestrator: &str) -> anyhow::Result<()> {
    // In production: send via TransferDaemon or HTTP
    info!("Sending result for job: {}", result.job_id);
    Ok(())
}

/// Check if shutdown signal was received
async fn should_shutdown() -> bool {
    // In production: check for signal, shutdown file, etc.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_receive_job() {
        let job = receive_job().await.unwrap();
        assert!(!job.src_lang.is_empty());
        assert!(!job.tgt_lang.is_empty());
    }

    #[tokio::test]
    async fn test_send_result() {
        let mut result = TestResult::default();
        result.job_id = "test-job-1".into();
        let send_result = send_result(&result, "127.0.0.1:9000").await;
        assert!(send_result.is_ok());
    }
}
