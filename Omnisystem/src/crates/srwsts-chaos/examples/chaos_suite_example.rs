//! Comprehensive chaos engineering example.
//!
//! Demonstrates running a complete chaos test suite with multiple scenarios.

use srwsts_chaos::{
    scenarios, suite_executor::{ChaosTestConfig, ChaosSuiteExecutor},
    weakness_prediction::{RecommendationGenerator, WeaknessPredictor},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("========================================");
    println!("SRWSTS Chaos Engineering Suite");
    println!("========================================\n");

    // Test 1: Black Friday Traffic Surge
    println!("Test 1: Black Friday Traffic Surge");
    println!("------------------------------------");

    let config = ChaosTestConfig {
        scenario: "Black Friday Traffic Surge".to_string(),
        num_runs: 20,
        base_seed: 1,
        max_detection_ms: 5000,
        max_recovery_ms: 30000,
        allow_data_loss: false,
        require_all_success: true,
        run_timeout_secs: 60,
    };

    let scenario = scenarios::scenario_black_friday()?;
    println!("Scenario: {}", scenario.name);
    println!("Description: {}", scenario.description);
    println!("Impact Level: {:?}", scenario.impact_level);
    println!("Faults to inject: {}", scenario.fault_schedule.fault_count());
    println!();

    let mut executor = ChaosSuiteExecutor::new(config);
    executor.run_suite(&scenario).await?;

    let results = executor.results();
    println!("Results:");
    println!("  Pass Rate: {}%", results.pass_rate_percent());
    println!("  Total Runs: {}", results.runs.len());
    println!("  Total Runtime: {}ms", results.total_runtime_ms());
    println!("  Avg Run Time: {}ms", results.avg_runtime_ms());

    // Analyze for weaknesses
    let analysis = WeaknessPredictor::analyze(&results);
    println!("\nWeakness Analysis:");
    println!("  Resilience Score: {}/100", analysis.overall_resilience_score);
    println!("  Weaknesses Found: {}", analysis.weaknesses.len());

    for weakness in &analysis.weaknesses {
        println!("\n  - {}", weakness.name);
        println!("    Component: {}", weakness.component);
        println!("    Severity: {}/10", weakness.severity);
        println!("    Confidence: {}%", weakness.confidence);
        println!("    Description: {}", weakness.description);
        println!("    Triggering Faults: {:?}", weakness.triggering_faults);
    }

    if !analysis.critical_path_issues.is_empty() {
        println!("\n  CRITICAL ISSUES:");
        for issue in &analysis.critical_path_issues {
            println!("    - {}", issue);
        }
    }

    // Generate recommendations
    let recommendations = RecommendationGenerator::generate(&analysis);
    if !recommendations.is_empty() {
        println!("\n  Top Recommendations:");
        for (i, rec) in recommendations.iter().take(5).enumerate() {
            println!("\n  {}. {} (Priority: {})", i + 1, rec.recommendation, rec.priority);
            println!("     Weakness: {}", rec.weakness);
            println!("     Component: {}", rec.component);
            println!("     Severity: {}/10, Confidence: {}%", rec.severity, rec.confidence);
            println!("     Effort: {:?}", rec.estimated_effort);
        }
    }

    println!("\n\nTest 2: Network Meltdown");
    println!("------------------------------------\n");

    let config2 = ChaosTestConfig {
        scenario: "Network Meltdown".to_string(),
        num_runs: 15,
        base_seed: 101,
        max_detection_ms: 3000,
        max_recovery_ms: 15000,
        allow_data_loss: false,
        require_all_success: true,
        run_timeout_secs: 60,
    };

    let scenario2 = scenarios::scenario_network_meltdown()?;
    println!("Scenario: {}", scenario2.name);
    println!("Impact Level: {:?}", scenario2.impact_level);
    println!("Faults to inject: {}", scenario2.fault_schedule.fault_count());

    let mut executor2 = ChaosSuiteExecutor::new(config2);
    executor2.run_suite(&scenario2).await?;

    let results2 = executor2.results();
    println!("\nResults:");
    println!("  Pass Rate: {}%", results2.pass_rate_percent());
    println!("  Total Runs: {}", results2.runs.len());
    println!("  Total Runtime: {}ms", results2.total_runtime_ms());

    println!("\n\nTest 3: Storage Corruption");
    println!("------------------------------------\n");

    let config3 = ChaosTestConfig {
        scenario: "Storage Corruption".to_string(),
        num_runs: 10,
        base_seed: 201,
        max_detection_ms: 8000,
        max_recovery_ms: 60000,
        allow_data_loss: false,
        require_all_success: true,
        run_timeout_secs: 120,
    };

    let scenario3 = scenarios::scenario_storage_corruption()?;
    println!("Scenario: {}", scenario3.name);
    println!("Impact Level: {:?}", scenario3.impact_level);

    let mut executor3 = ChaosSuiteExecutor::new(config3);
    executor3.run_suite(&scenario3).await?;

    let results3 = executor3.results();
    println!("\nResults:");
    println!("  Pass Rate: {}%", results3.pass_rate_percent());
    println!("  Total Runs: {}", results3.runs.len());

    // Summary across all tests
    println!("\n\n========================================");
    println!("Overall Summary");
    println!("========================================\n");

    let all_pass_rate = (
        (results.pass_rate_percent() + results2.pass_rate_percent() + results3.pass_rate_percent())
            as u32
            / 3
    );

    println!("Tests Executed: 3");
    println!("Overall Pass Rate: {}%", all_pass_rate);
    println!("Total Faults Injected: {}",
        scenario.fault_schedule.fault_count() +
        scenario2.fault_schedule.fault_count() +
        scenario3.fault_schedule.fault_count()
    );
    println!("Total Runtime: {}ms + {}ms + {}ms = {}ms",
        results.total_runtime_ms(),
        results2.total_runtime_ms(),
        results3.total_runtime_ms(),
        results.total_runtime_ms() + results2.total_runtime_ms() + results3.total_runtime_ms()
    );

    if all_pass_rate >= 90 {
        println!("\n✓ System demonstrates EXCELLENT resilience");
    } else if all_pass_rate >= 70 {
        println!("\n⚠ System demonstrates GOOD resilience but has areas for improvement");
    } else {
        println!("\n✗ System resilience needs SIGNIFICANT improvement");
    }

    Ok(())
}
