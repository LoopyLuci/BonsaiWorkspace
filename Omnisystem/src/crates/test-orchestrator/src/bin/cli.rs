//! UTOF CLI - Universal Test Orchestration Fabric
//!
//! Loads a test specification (TOML) and runs it end to end through the
//! real Orchestrator: scheduling jobs, spawning per-language subprocesses,
//! comparing output fidelity, and storing/reporting results.

use std::path::PathBuf;
use test_orchestrator::{Orchestrator, TestSpec, UtofConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("specs/addition.toml"));

    println!("UTOF - Universal Test Orchestration Fabric");
    println!("Loading spec from: {}", spec_path.display());

    let spec = TestSpec::load(&spec_path)?;
    spec.validate()?;

    println!("Loaded spec: {} ({})", spec.name, spec.description);
    println!("Reference language: {}", spec.reference_lang);
    println!("Languages to test: {}", spec.languages.join(", "));
    println!("Test cases: {}", spec.test_cases.len());

    let work_dir = std::env::temp_dir().join("utof-workspace");
    let config = UtofConfig::new(work_dir)?;
    let mut orchestrator = Orchestrator::new(config)?;

    let stats = orchestrator.run_spec(&spec).await?;

    println!("\n================ TEST SUITE RESULTS ================");
    println!("  Suite:         {}", stats.spec_name);
    println!("  Total Tests:   {}", stats.total_tests);
    println!("  Passed:        {}", stats.passed);
    println!("  Failed:        {}", stats.failed);
    println!("  Success Rate:  {:.1}%", stats.success_rate);
    println!("  Avg Fidelity:  {:.3}", stats.avg_fidelity);
    println!("  Total Time:    {}ms", stats.total_execution_time_ms);
    println!("======================================================\n");

    if stats.failed == 0 && stats.total_tests > 0 {
        println!("All tests passed.");
        Ok(())
    } else if stats.total_tests == 0 {
        println!("No test cases were defined in the spec.");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "{} of {} test(s) failed (avg fidelity {:.3})",
            stats.failed,
            stats.total_tests,
            stats.avg_fidelity
        ))
    }
}
