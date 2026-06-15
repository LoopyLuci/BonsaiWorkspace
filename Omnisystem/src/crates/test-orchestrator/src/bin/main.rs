/// UTOF CLI - Universal Test Orchestration Fabric
use bonsai_utof::{Orchestrator, UtofConfig, TestSpec};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "utof",
    about = "Universal Test Orchestration Fabric - Deterministic polyglot test harness",
    version = "0.1.0"
)]
struct Args {
    /// Path to test specification TOML file
    #[arg(long, short)]
    spec: PathBuf,

    /// Output results as JSON (optional)
    #[arg(long)]
    output_json: Option<PathBuf>,

    /// Output results as CSV (optional)
    #[arg(long)]
    output_csv: Option<PathBuf>,

    /// Working directory for test artifacts
    #[arg(long, default_value = "./utof-workspace")]
    work_dir: PathBuf,

    /// Maximum concurrent tests
    #[arg(long, default_value = "10")]
    max_workers: usize,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(level.parse()?))
        .init();

    tracing::info!("UTOF - Universal Test Orchestration Fabric");
    tracing::info!("Loading spec from: {}", args.spec.display());

    // Load test specification
    let spec = TestSpec::load(&args.spec)?;
    spec.validate()?;

    tracing::info!("Loaded spec: {} ({})", spec.name, spec.description);
    tracing::info!("Reference language: {}", spec.reference_lang);
    tracing::info!("Languages to test: {}", spec.languages.join(", "));
    tracing::info!("Test cases: {}", spec.test_cases.len());

    // Create orchestrator
    let config = UtofConfig::new(args.work_dir)?;
    let mut orchestrator = Orchestrator::new(config)?;

    // Run the test suite
    let stats = orchestrator.run_spec(&spec).await?;

    // Print summary
    println!("\n════════════════════════════════════════════════════════════");
    println!("  TEST SUITE RESULTS");
    println!("════════════════════════════════════════════════════════════");
    println!("  Suite:               {}", stats.spec_name);
    println!("  Total Tests:         {}", stats.total_tests);
    println!("  Passed:              {} ✓", stats.passed);
    println!("  Failed:              {} ✗", stats.failed);
    println!("  Success Rate:        {:.1}%", stats.success_rate);
    println!("  Avg Fidelity:        {:.3}", stats.avg_fidelity);
    println!("  Total Time:          {}ms", stats.total_execution_time_ms);
    println!("════════════════════════════════════════════════════════════\n");

    if stats.failed == 0 && stats.avg_fidelity >= 0.99 {
        println!("✓ ALL TESTS PASSED WITH PERFECT FIDELITY");
    } else {
        println!("⚠ {} test(s) did not pass or had reduced fidelity", stats.failed);
    }

    // Export results if requested
    if let Some(json_path) = args.output_json {
        let store = orchestrator.results();
        let json = store.export_json();
        std::fs::write(&json_path, serde_json::to_string_pretty(&json)?)?;
        tracing::info!("JSON results exported to: {}", json_path.display());
    }

    if let Some(csv_path) = args.output_csv {
        let store = orchestrator.results();
        let csv = store.export_csv();
        std::fs::write(&csv_path, csv)?;
        tracing::info!("CSV results exported to: {}", csv_path.display());
    }

    // Return success/failure based on test results
    if stats.failed == 0 && stats.avg_fidelity >= 0.99 {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Tests failed: {} failures, avg fidelity {:.3}",
            stats.failed,
            stats.avg_fidelity
        ))
    }
}
