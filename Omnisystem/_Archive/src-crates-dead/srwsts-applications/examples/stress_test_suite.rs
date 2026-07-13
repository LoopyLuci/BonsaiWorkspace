//! Complete example demonstrating the SRWSTS Applications stress testing suite

use srwsts_applications::{
    ApplicationStressConfig, ApplicationStressEnvironment, ApplicationTestRunner, TestContext,
    FaultScenarioExecutor, FaultScenario, InteractionScenarioExecutor, InteractionScenario,
};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().init();

    println!("=== Bonsai Ecosystem Application Stress Testing Suite ===\n");

    // 1. Configure stress testing environment
    println!("1. Configuring test environment...");
    let config = ApplicationStressConfig {
        max_concurrent_apps: 10,
        max_concurrent_users: 100,
        test_timeout_secs: 600,
        verbose: true,
        store_artifacts: true,
        artifact_dir: PathBuf::from("./stress-test-artifacts"),
        profile_performance: true,
        enable_fault_injection: true,
        memory_monitor_interval_ms: 500,
        deterministic: false,
    };

    // 2. Create testing environment
    println!("2. Creating testing environment...");
    let env = ApplicationStressEnvironment::new(config.clone()).await?;
    env.initialize().await?;

    let metrics = env.metrics.clone();
    let state_handle = env.state.clone();

    // 3. Create test runner with all application stress tests
    println!("3. Setting up application stress tests...");
    let runner = ApplicationTestRunner::new();

    println!(
        "   Available tests: {}",
        runner
            .list_tests()
            .iter()
            .map(|(_, name)| name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // 4. Create test context
    println!("4. Creating test execution context...");
    let test_context = TestContext::new(
        "stress-test-run-001",
        "full-suite",
        metrics.clone(),
        config.artifact_dir.clone(),
        config.test_timeout_secs,
    );

    // 5. Run all application stress tests
    println!("\n5. Executing stress tests...\n");
    let results = runner.run_all(&test_context).await?;

    // Display individual test results
    println!("=== Test Results ===");
    for result in &results {
        println!(
            "  [{}] {} - {} ({}ms)",
            result.status, result.name, result.status, result.duration_ms
        );
        if let Some(metrics) = &result.metrics {
            println!("       Metrics: {}", metrics);
        }
    }

    // 6. Aggregate and display results
    println!("\n=== Aggregate Results ===");
    let aggregate = ApplicationTestRunner::aggregate_results(&results);
    println!("{}", aggregate.summary());
    println!(
        "Pass rate: {:.2}%\n",
        aggregate.pass_rate()
    );

    // 7. Run fault injection scenarios
    println!("=== Fault Injection Scenarios ===\n");

    let fault_scenarios = vec![
        FaultScenario::app_crash("workspace"),
        FaultScenario::network_loss("buddy"),
        FaultScenario::storage_corruption("workspace"),
        FaultScenario::gpu_reset(),
        FaultScenario::memory_exhaustion("omnibot"),
    ];

    for scenario in fault_scenarios {
        println!(
            "Running fault scenario: {} on {}",
            scenario.fault_type, scenario.affected_app
        );
        match FaultScenarioExecutor::execute(&scenario).await {
            Ok(result) => {
                println!(
                    "  Result: {} (duration: {}ms, recovery: {}ms)",
                    if result.success { "SUCCESS" } else { "DEGRADED" },
                    result.duration_ms,
                    result.recovery_time_ms.unwrap_or(0)
                );
            }
            Err(e) => {
                println!("  Failed: {}", e);
            }
        }
    }

    // 8. Run cross-application interaction scenarios
    println!("\n=== Cross-Application Interaction Scenarios ===\n");

    let interaction_scenarios = vec![
        InteractionScenario::workspace_buddy_sync(),
        InteractionScenario::buddy_omnibot_query(),
        InteractionScenario::fullstack_integration(),
        InteractionScenario::cascading_failure(),
    ];

    for scenario in interaction_scenarios {
        println!(
            "Running interaction scenario: {} between {:?}",
            scenario.scenario_type, scenario.apps_involved
        );
        match InteractionScenarioExecutor::execute(&scenario).await {
            Ok(result) => {
                println!(
                    "  Result: {} (duration: {}ms)",
                    if result.success { "SUCCESS" } else { "FAILED" },
                    result.duration_ms
                );
            }
            Err(e) => {
                println!("  Failed: {}", e);
            }
        }
    }

    // 9. Display metrics summary
    println!("\n=== Performance Metrics Summary ===");
    let metrics_summary = metrics.summary();
    println!(
        "Elapsed time: {:.2}s",
        metrics_summary.elapsed_secs
    );
    println!(
        "Peak memory: {} MB, Average memory: {} MB",
        metrics_summary.memory.peak_mb, metrics_summary.memory.average_mb
    );
    println!(
        "UI responsiveness: {:.2}ms average frame time (p99: {:.2}ms)",
        metrics_summary.ui.avg_frame_time_ms, metrics_summary.ui.p99_frame_time_ms
    );
    println!(
        "Compilation performance: {} compilations, {:.2}ms average",
        metrics_summary.performance.compilations,
        metrics_summary.performance.avg_compilation_ms
    );

    // 10. Cleanup
    println!("\n=== Cleanup ===");
    env.shutdown().await?;

    println!("\n=== Stress Testing Complete ===");

    Ok(())
}
