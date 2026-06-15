# Usage Examples: Bonsai Adaptive Benchmarks

## Example 1: Running Unit Tests

### Test Layer Masking
```rust
#[test]
fn test_layer_mask_skip_connections() {
    use bonsai_adaptive_benchmarks::unit_tests::LayerMask;
    
    let mask = LayerMask::with_pattern(100, &[0, 50, 99]);
    assert_eq!(mask.active_layers(), 3);
    
    // Verify that skip connections work correctly
    let batch_size = 1;
    let seq_len = 512;
    let hidden = 256;
    
    // In real code: output_masked should have same shape as input
    // with proper skip connection handling through inactive layers
}
```

### Run Masking Tests
```bash
cargo test -p bonsai-adaptive-benchmarks unit_tests::layer_masking_tests
```

## Example 2: Performance Benchmarking

### Basic Latency Benchmark
```rust
use bonsai_adaptive_benchmarks::performance::{
    PerformanceBenchmark, PerformanceMetrics
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let benchmark = PerformanceBenchmark::new(
        vec![100_000_000, 1_000_000_000, 7_000_000_000],
        vec![1, 8, 32],
        vec![512],
        vec!["gpu".to_string()],
    );
    
    // Measure latency
    let metrics = benchmark.benchmark_latency().await;
    
    for m in metrics {
        println!(
            "Scale: {}, TTFT: {:.2}ms, TPT: {:.2}ms, Throughput: {:.2} tok/s",
            m.scale, m.ttft_ms, m.tpt_ms, m.throughput_tokens_sec
        );
    }
    
    Ok(())
}
```

### Run Performance Benchmarks
```bash
cargo bench -p bonsai-adaptive-benchmarks --bench performance_bench
```

## Example 3: Correctness Testing

### Test Subset Validity
```rust
use bonsai_adaptive_benchmarks::correctness::CorrectnessTest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let test = CorrectnessTest::new(
        "subset_validity".to_string(),
        vec![
            100_000_000,
            500_000_000,
            1_000_000_000,
            7_000_000_000,
            30_000_000_000,
            100_000_000_000,
        ],
        100,  // 100 test samples
    );
    
    let result = test.test_subset_validity().await;
    
    if result.passed {
        println!("✓ Subset validity verified!");
    } else {
        println!("✗ Subset validity failed:");
        for failure in result.failed_samples {
            println!("  - {}", failure);
        }
    }
    
    println!("Avg KL divergence: {:.4}", 
             result.consistency_metrics.avg_kl_divergence);
    println!("Hallucination rate: {:.2}%",
             result.consistency_metrics.hallucination_rate * 100.0);
    
    Ok(())
}
```

## Example 4: Regression Detection

### Detect Regressions Between Versions
```rust
use bonsai_adaptive_benchmarks::regression::RegressionDetector;
use std::collections::HashMap;

fn main() {
    let detector = RegressionDetector::new(5.0);  // 5% threshold
    
    // Baseline metrics (v0.1.0)
    let mut baseline = HashMap::new();
    baseline.insert("latency_100m".to_string(), 15.5);
    baseline.insert("latency_1b".to_string(), 25.3);
    baseline.insert("mmlu_score".to_string(), 80.0);
    baseline.insert("perplexity".to_string(), 25.3);
    
    // Current metrics (v0.2.0)
    let mut current = HashMap::new();
    current.insert("latency_100m".to_string(), 15.8);   // 1.9% increase
    current.insert("latency_1b".to_string(), 27.2);    // 7.5% increase - REGRESSION
    current.insert("mmlu_score".to_string(), 78.5);    // 1.9% decrease
    current.insert("perplexity".to_string(), 26.1);    // 3.2% increase
    
    let report = detector.detect_all_regressions(&baseline, &current);
    
    println!("Regressions detected: {}", !report.all_metrics_passed);
    println!("Severity: {:.2}", report.regression_severity);
    
    for regression in &report.regressions_detected {
        println!(
            "  {} {}: {:.1}% (severity: {})",
            regression.metric_name,
            if regression.regression_pct > 0.0 { "↑" } else { "↓" },
            regression.regression_pct.abs(),
            regression.severity
        );
    }
    
    // Decide if rollback needed
    if report.regression_severity > 0.7 {
        println!("⚠️ Severe regression detected! Consider rollback.");
    }
}
```

## Example 5: Using Test Fixtures

### Run Benchmark on Standard Dataset
```rust
use bonsai_adaptive_benchmarks::test_fixtures::TestFixture;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load standard benchmark suite
    let fixture = TestFixture::benchmark_suite();
    
    println!("Running benchmarks on {} samples", fixture.total_samples());
    
    for dataset in &fixture.datasets {
        println!("\nDataset: {} ({} samples)", dataset.name, dataset.num_samples);
        
        // Run inference on first 5 prompts as example
        for (i, prompt) in dataset.prompts.iter().take(5).enumerate() {
            println!("  [{}] {}", i + 1, &prompt[..50.min(prompt.len())]);
        }
    }
    
    Ok(())
}
```

### Create Custom Fixture
```rust
use bonsai_adaptive_benchmarks::test_fixtures::{TestDataset, TestFixture};

fn main() {
    let mut custom = TestFixture::new("my_custom_benchmark".to_string(), 42);
    
    // Add specific datasets
    custom.datasets.push(TestDataset::small_general());
    custom.datasets.push(TestDataset::medium_code());
    custom.datasets.push(TestDataset::humaneval_benchmark());
    
    println!("Custom fixture ready with {} samples", custom.total_samples());
}
```

## Example 6: Benchmark Orchestration

### Run Comprehensive Benchmark Suite
```rust
use bonsai_adaptive_benchmarks::benchmarking::{BenchmarkConfig, BenchmarkRunner};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = BenchmarkConfig {
        name: "full_adaptive_benchmark".to_string(),
        description: "Comprehensive benchmark of adaptive transformer".to_string(),
        scales: vec![
            100_000_000,
            500_000_000,
            1_000_000_000,
            7_000_000_000,
        ],
        batch_sizes: vec![1, 8, 32],
        sequence_lengths: vec![128, 512, 2048],
        devices: vec!["cpu".to_string(), "gpu".to_string()],
        num_runs: 5,
        warmup_runs: 2,
        output_dir: PathBuf::from("./benchmark_results"),
        enable_profiling: true,
        enable_memory_tracking: true,
        ..Default::default()
    };
    
    let mut runner = BenchmarkRunner::new(config);
    runner.run_all().await?;
    
    // Generate report
    let report = runner.generate_report();
    println!("Benchmark complete! {} results generated", report.results_count);
    
    // Find bottlenecks
    let bottlenecks = runner.find_bottlenecks();
    if !bottlenecks.is_empty() {
        println!("\nBottlenecks detected:");
        for b in bottlenecks {
            println!("  - {}: {} (severity: {})", 
                     b.location, b.reason, b.severity);
        }
    }
    
    Ok(())
}
```

## Example 7: Formal Verification

### Verify Critical Properties
```rust
use bonsai_adaptive_benchmarks::formal_verification::FormalVerifier;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let verifier = FormalVerifier::new();
    
    // Run all verifications
    let report = verifier.verify_all().await;
    
    println!("Formal Verification Report");
    println!("==========================");
    println!("Status: {}", 
             if report.all_properties_verified { "✓ PASSED" } else { "✗ FAILED" });
    println!("Confidence: {:.1}%", report.avg_confidence_level * 100.0);
    println!();
    
    for result in &report.results {
        let status = if result.verified { "✓" } else { "✗" };
        println!("{} {} (confidence: {:.0}%)",
                 status, result.property_name, result.confidence_level * 100.0);
        println!("  Proof: {}", &result.proof_sketch[..100.min(result.proof_sketch.len())]);
    }
    
    Ok(())
}
```

## Example 8: Observability & Metrics

### Record and Export Metrics
```rust
use bonsai_adaptive_benchmarks::observability::{
    MetricsCollector, BenchmarkLogger
};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let metrics = Arc::new(MetricsCollector::new());
    let logger = Arc::new(BenchmarkLogger::new());
    
    // Simulate inference and record metrics
    for scale in &[100_000_000, 1_000_000_000, 7_000_000_000] {
        metrics.record_latency(15.5, *scale, 1);
        metrics.record_memory(2048.0, *scale);
        metrics.record_perplexity(25.3, *scale);
        
        let mut ctx = HashMap::new();
        ctx.insert("scale".to_string(), format!("{}", scale));
        logger.info(&format!("Completed inference at scale {}", scale), ctx);
    }
    
    // Aggregate and display
    let aggregated = metrics.aggregate_metrics();
    for (name, metric) in &aggregated {
        println!("{}: mean={:.2}, std={:.2}, min={:.2}, max={:.2}",
                 name, metric.mean, metric.std_dev, metric.min, metric.max);
    }
    
    // Export results
    let metrics_json = metrics.export_json();
    std::fs::write("metrics.json", metrics_json)?;
    
    let logs_json = logger.export_logs();
    std::fs::write("logs.json", logs_json)?;
    
    println!("\n✓ Metrics and logs exported");
    
    Ok(())
}
```

## Example 9: Integration with Production Code

### Add Metrics to InferenceEngine
```rust
use bonsai_adaptive_benchmarks::observability::MetricsCollector;
use std::sync::Arc;

pub struct InferenceEngineWithMetrics {
    metrics: Arc<MetricsCollector>,
    // ... other fields
}

impl InferenceEngineWithMetrics {
    pub async fn generate_with_metrics(
        &self,
        prompt: &str,
        scale: u32,
    ) -> anyhow::Result<String> {
        let start = std::time::Instant::now();
        
        // Run inference
        let output = self.generate(prompt, scale).await?;
        
        // Record metrics
        let elapsed = start.elapsed().as_secs_f32() * 1000.0;
        self.metrics.record_latency(elapsed, scale, 1);
        
        Ok(output)
    }
    
    pub async fn generate_stream_with_metrics(
        &self,
        prompt: &str,
        scale: u32,
    ) -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        
        // Stream tokens
        self.generate_stream(prompt, scale).await?;
        
        // Record
        let elapsed = start.elapsed().as_secs_f32() * 1000.0;
        self.metrics.record_latency(elapsed, scale, 1);
        
        Ok(())
    }
}
```

## Example 10: CI/CD Integration

### GitHub Actions Workflow
```yaml
name: Adaptive Benchmarks

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test -p bonsai-adaptive-benchmarks --lib
      
  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench -p bonsai-adaptive-benchmarks --bench performance_bench
      
  regression:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test -p bonsai-adaptive-benchmarks regression::
```

## Troubleshooting Examples

### High Variance in Benchmarks
```rust
// Problem: CV > 0.2 (high variability)
// Solution: Increase sample count and warmup runs

let config = BenchmarkConfig {
    warmup_runs: 5,      // Was 2
    num_runs: 20,        // Was 5
    ..Default::default()
};
```

### Regression Threshold Too Strict
```rust
// Problem: Too many false positives
// Solution: Adjust threshold

let detector = RegressionDetector::new(10.0);  // Was 5.0 (10% instead)
let report = detector.detect_all_regressions(&baseline, &current);
```

### Memory Leak in Benchmarks
```bash
# Problem: Memory growing unbounded
# Solution: Profile with valgrind

valgrind --leak-check=full cargo test -p bonsai-adaptive-benchmarks
```

## Complete Example: Full Workflow

```rust
use bonsai_adaptive_benchmarks::{
    benchmarking::*, correctness::*, regression::*,
    observability::*, formal_verification::*,
    test_fixtures::*,
};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Setup
    let metrics = Arc::new(MetricsCollector::new());
    let logger = Arc::new(BenchmarkLogger::new());
    
    // 2. Run correctness tests
    println!("1. Running correctness tests...");
    let correct = CorrectnessTest::new(
        "comprehensive".to_string(),
        vec![100_000_000, 1_000_000_000, 7_000_000_000],
        100,
    );
    let correct_result = correct.test_subset_validity().await;
    println!("   ✓ Passed: {}", correct_result.passed);
    
    // 3. Run formal verification
    println!("2. Formal verification...");
    let verifier = FormalVerifier::new();
    let verify_result = verifier.verify_all().await;
    println!("   ✓ All properties verified");
    
    // 4. Run performance benchmarks
    println!("3. Performance benchmarking...");
    let config = BenchmarkConfig::default();
    let mut runner = BenchmarkRunner::new(config);
    runner.run_all().await?;
    println!("   ✓ {} results", runner.results.len());
    
    // 5. Check for regressions
    println!("4. Regression detection...");
    let detector = RegressionDetector::new(5.0);
    let mut baseline = HashMap::new();
    baseline.insert("latency_1b".to_string(), 25.0);
    let mut current = HashMap::new();
    current.insert("latency_1b".to_string(), 25.5);
    let report = detector.detect_all_regressions(&baseline, &current);
    println!("   ✓ Regressions: {}", report.regressions_detected.len());
    
    // 6. Export results
    println!("5. Exporting results...");
    let metrics_json = metrics.export_json();
    std::fs::write("metrics.json", metrics_json)?;
    println!("   ✓ Saved to metrics.json");
    
    println!("\n✓ Full workflow completed successfully!");
    Ok(())
}
```

These examples cover all major use cases of the benchmarking framework!
