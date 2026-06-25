# Bonsai Adaptive Transformer: Benchmarking & Testing Framework

## Overview

This framework provides comprehensive testing, validation, and benchmarking for the Bonsai Adaptive Transformer. It ensures that scaling is safe, efficient, and produces quality results across all parameter ranges.

## Architecture

### 1. Core Modules

#### `unit_tests.rs`
Tests for individual components:
- **Layer Masking**: Verify masks work correctly, skip connections bypass inactive layers
- **Width Scaling**: Test dimension truncation and projection matrices
- **Expert Routing**: Ensure balanced token distribution
- **LoRA Adapters**: Test composition and scaling
- **KV-Cache**: Verify invalidation and correctness
- **Gradient Flow**: Check backpropagation through masked layers

**Key Test Types**:
- Determinism tests (same input/seed = same output)
- Shape preservation tests
- Skip connection correctness
- Expert load balancing
- Gradient flow continuity

#### `correctness.rs`
Tests for model correctness across scales:
- **Subset Validity**: Outputs at smaller scales similar to full scale
- **KL Divergence**: Bounded and quantified
- **Hallucination Detection**: Prevents mode collapse
- **Semantic Consistency**: Outputs maintain semantic meaning

**Methods**:
```rust
test_subset_validity()        // Smaller models are proper subsets
test_kl_divergence_bounded()  // KL(P_small, P_large) < threshold
test_hallucination_rates()    // Detect confabulation
```

#### `performance.rs`
Benchmarking suite for:
- **Latency**: TTFT (time to first token), TPT (time per token)
- **Throughput**: Tokens/sec, requests/sec
- **Memory**: Peak GPU, KV-cache, model memory
- **Energy**: Estimated watt-hours per inference
- **Quality**: Perplexity, MMLU, HumanEval scores

**Benchmark Methods**:
```rust
benchmark_latency()        // Measure TTFT and TPT
benchmark_memory()         // Profile memory usage
benchmark_quality()        // Run quality benchmarks
benchmark_throughput()     // Measure tokens/sec
```

#### `regression.rs`
Automated regression detection:
- **Latency Regressions**: Alert if latency increases >5%
- **Quality Regressions**: Alert if scores decrease >5%
- **Auto-Rollback**: Revert to previous version on severe regressions
- **Detailed Reporting**: Track severity and trends

**Methods**:
```rust
detect_all_regressions()      // Find all metric regressions
should_rollback()             // Decide if rollback needed
perform_rollback()            // Execute rollback
```

#### `test_fixtures.rs`
Deterministic test data:
- **Dataset Sizes**: Small (100), Medium (1K), Large (10K)
- **Domains**: Code, Math, General Knowledge, Reasoning, Creative
- **Benchmarks**: MMLU, HumanEval, WikiText, OpenWebText
- **Deterministic Generation**: Same seed = same data

**Available Fixtures**:
```rust
TestDataset::small_general()       // 100 general knowledge prompts
TestDataset::medium_code()         // 1000 code problems
TestDataset::humaneval_benchmark() // 164 standard HumanEval
TestDataset::mmlu_benchmark()      // ~14K MMLU questions
TestFixture::benchmark_suite()     // Full benchmark set
```

#### `benchmarking.rs`
Orchestration of benchmark runs:
- **Configuration**: Scale ranges, batch sizes, sequence lengths
- **Statistical Analysis**: Compute mean, std dev, percentiles
- **Bottleneck Detection**: Find high-variability configs
- **Report Generation**: Summarize results

**Key Classes**:
```rust
BenchmarkConfig              // Configuration
BenchmarkRunner              // Orchestrator
BenchmarkResult              // Single result
RunStatistics               // Statistical summary
BenchmarkReport             // Full report
```

#### `formal_verification.rs`
Formal proofs of critical properties:
- **Subset Validity**: Smaller models ⊂ larger models
- **KL Divergence Bounds**: Provable limits
- **Determinism**: Same seed = same output
- **Skip Connection Correctness**: Proper bypass of masked layers
- **Property-Based Testing**: Generate random masks and verify

**Verified Properties**:
```rust
verify_subset_validity()
verify_kl_divergence_bounded()
verify_determinism()
verify_skip_connections()
verify_with_property_tests()  // Proptest/Hypothesis integration
```

#### `observability.rs`
Metrics collection and logging:
- **MetricsCollector**: Centralized metrics recording
- **BenchmarkLogger**: Structured logging
- **DashboardBuilder**: Aggregate for monitoring
- **CostAnalysis**: Track spending per scale

**Key Interfaces**:
```rust
MetricsCollector::record_latency()
MetricsCollector::record_memory()
MetricsCollector::record_perplexity()
BenchmarkLogger::info(), warn(), error(), debug()
DashboardData::build()
```

### 2. Benchmark Suites

#### `benches/adaptive_scale_bench.rs`
Criterion benchmarks for scaling operations:
- Layer masking (10, 50, 100 layers)
- Width scaling (256→128, 512→256, etc.)
- Expert routing (load balancing)
- KV-cache operations
- Scale transitions (100M→1B, etc.)

**Run with**:
```bash
cargo bench --bench adaptive_scale_bench
```

#### `benches/performance_bench.rs`
End-to-end performance benchmarking:
- Latency across scales (100M, 1B, 7B)
- Memory usage across configs
- Throughput (tokens/sec)
- Quality metrics (perplexity, MMLU)
- End-to-end inference pipelines

#### `benches/correctness_bench.rs`
Benchmarking correctness operations:
- KL divergence computation
- Output consistency checking
- Hallucination detection
- Subset validation
- Semantic similarity

#### `benches/regression_bench.rs`
Regression detection benchmarks:
- Detecting regressions in metrics
- Latency regression checks
- Quality regression checks
- Rollback decision logic
- Report generation

## Running the Framework

### Unit Tests
```bash
cargo test -p bonsai-adaptive-benchmarks
```

### Specific Test Module
```bash
cargo test -p bonsai-adaptive-benchmarks unit_tests::
cargo test -p bonsai-adaptive-benchmarks correctness::
cargo test -p bonsai-adaptive-benchmarks formal_verification::
```

### Benchmarks
```bash
cargo bench -p bonsai-adaptive-benchmarks
cargo bench -p bonsai-adaptive-benchmarks --bench adaptive_scale_bench
cargo bench -p bonsai-adaptive-benchmarks --bench performance_bench
```

### With Profiling
```bash
cargo bench -p bonsai-adaptive-benchmarks --bench performance_bench -- --profile-time=10
```

### Generate Regression Report
```rust
let detector = RegressionDetector::new(5.0);
let baseline = load_baseline_metrics();
let current = run_benchmarks();
let report = detector.detect_all_regressions(&baseline, &current);

if detector.should_rollback(&report) {
    perform_rollback(&report.baseline_version).await?;
}
```

## Test Data & Fixtures

### Available Datasets

| Name | Size | Domain | Samples |
|------|------|--------|---------|
| small_general | Small | General Knowledge | 100 |
| medium_code | Medium | Code | 1000 |
| large_reasoning | Large | Reasoning | 10000 |
| wikitext_validation | Medium | General Knowledge | 1000 |
| humaneval_benchmark | Small | Code | 164 |
| mmlu_benchmark | Medium | MMLU Questions | 14000 |

### Using Fixtures
```rust
let fixture = TestFixture::benchmark_suite();
for dataset in &fixture.datasets {
    run_benchmark_on_dataset(dataset).await?;
}
```

## Success Criteria

### Correctness
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] KL divergence < 0.15 across all scales
- [ ] Hallucination rate < 5%
- [ ] Formal verification passes with >90% confidence

### Performance
- [ ] TTFT < 100ms at 7B scale
- [ ] TPT < 50ms per token
- [ ] Memory scaling is O(n) with scale
- [ ] Throughput > 50 tokens/sec

### Quality
- [ ] Perplexity maintained within 5% of baseline
- [ ] MMLU score maintained within 5% of baseline
- [ ] HumanEval pass@1 maintained within 5% of baseline

### Safety
- [ ] Zero regressions on critical metrics
- [ ] Automatic rollback on severe regression
- [ ] All operations deterministic
- [ ] No NaN or Inf propagation

### Observability
- [ ] Every inference logged
- [ ] Every scaling operation logged
- [ ] Dashboard updated real-time
- [ ] Cost analysis accurate

## Integration with CI/CD

### Pre-Commit
```bash
cargo test -p bonsai-adaptive-benchmarks --lib
cargo fmt -p bonsai-adaptive-benchmarks --check
cargo clippy -p bonsai-adaptive-benchmarks -- -D warnings
```

### Nightly
```bash
cargo bench -p bonsai-adaptive-benchmarks --bench performance_bench
cargo bench -p bonsai-adaptive-benchmarks --bench correctness_bench
```

### Weekly
```bash
cargo bench -p bonsai-adaptive-benchmarks  # All benchmarks
# Compare against baseline
# Update dashboard
# Send regression alerts if needed
```

## Configuration

Default `BenchmarkConfig`:
```rust
scales: [100M, 500M, 1B, 7B, 30B, 100B]
batch_sizes: [1, 8, 32, 128]
sequence_lengths: [128, 512, 2048, 4096]
devices: [cpu, gpu, tpu]
num_samples: 100
regression_threshold_pct: 5.0
warmup_runs: 2
num_runs: 5
```

## Troubleshooting

### High Latency Variance (CV > 0.2)
- Check for competing processes
- Increase sample count
- Check thermal throttling
- Verify consistent batch sizes

### Regression Detected
1. Check commit diff
2. Run regression report
3. Decide: fix or adjust threshold
4. Update baseline if acceptable

### Hallucination Detection
- Check prompt quality
- Increase model scale
- Verify tokenizer consistency
- Review few-shot examples

## Future Enhancements

1. **Distributed Testing**: Run benchmarks across multiple GPUs/TPUs
2. **Continuous Monitoring**: Real-time dashboard with alerts
3. **Cost Optimization**: Auto-recommend optimal scale for workload
4. **A/B Testing**: Compare different scaling strategies
5. **Multi-Device**: Support mobile, desktop, server devices
6. **Automated Tuning**: Use Bayesian optimization for hyperparameters
7. **Custom Benchmarks**: User-defined quality metrics
8. **Trace Analysis**: Detailed causal debugging of failures

## References

- Hypothesis/Proptest: Property-based testing
- Criterion: Benchmarking framework
- ndarray: Tensor operations
- Burn/tch-rs: ML framework integration
