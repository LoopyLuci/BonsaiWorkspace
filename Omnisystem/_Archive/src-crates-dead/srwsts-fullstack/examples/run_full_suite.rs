//! Example: Run complete full-stack test suite
//!
//! This example demonstrates how to run the entire Omnisystem testing suite,
//! including bootstrap, nominal load, peak load, cascading failures, recovery,
//! network partitions, state consistency, end-to-end journeys, and long-duration tests.
//!
//! Usage: cargo run --example run_full_suite --release

use srwsts_fullstack::{
    bootstrap::BootstrapBuilder,
    runner::FullStackTestRunner,
    reporter::TestReporter,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  SRWSTS Full-Stack Integrated Testing System               ║");
    println!("║  Omnisystem (UOSC + Services + Applications)               ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Phase 1: Bootstrap
    println!("[PHASE 1] Bootstrapping complete system...");
    let bootstrap = BootstrapBuilder::new()
        .kernel_threads(num_cpus::get() as u32)
        .max_services(50)
        .max_applications(100)
        .verbose(false)
        .build();

    let vault = Arc::new(bootstrap.initialize().await?);
    println!("✓ Bootstrap complete\n");

    // Phase 2: Run all tests
    println!("[PHASE 2] Running comprehensive test suite...");
    let runner = FullStackTestRunner::new(vault.clone());
    let results = runner.run_all_tests().await?;

    // Phase 3: Generate report
    println!("[PHASE 3] Generating comprehensive report...");
    let report = TestReporter::generate_report(&results, &vault);

    // Display results
    println!("\n{}", results.summary());

    // Generate text report
    println!("\n{}", TestReporter::format_text_report(&report));

    // Exit with appropriate code
    if results.all_passed() {
        println!("\n✓ All tests passed!");
        Ok(())
    } else {
        println!(
            "\n✗ {} test(s) failed",
            results.total_tests_failed
        );
        anyhow::bail!("Test suite failed")
    }
}
