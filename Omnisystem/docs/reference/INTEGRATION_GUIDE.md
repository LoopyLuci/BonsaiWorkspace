# Integration Guide: Bonsai Adaptive Benchmarks

## Quick Start

### 1. Add to Workspace
Already added to `Cargo.toml` workspace members:
```toml
[workspace]
members = [
  ...
  "crates/bonsai-adaptive-benchmarks",
]
```

### 2. Run Tests
```bash
# Unit tests only
cargo test -p bonsai-adaptive-benchmarks --lib

# Integration tests
cargo test -p bonsai-adaptive-benchmarks --test integration

# All tests
cargo test -p bonsai-adaptive-benchmarks
```

### 3. Run Benchmarks
```bash
# Adaptive scaling benchmarks
cargo bench -p bonsai-adaptive-benchmarks --bench adaptive_scale_bench

# Performance benchmarks
cargo bench -p bonsai-adaptive-benchmarks --bench performance_bench

# Correctness benchmarks
cargo bench -p bonsai-adaptive-benchmarks --bench correctness_bench

# Regression benchmarks
cargo bench -p bonsai-adaptive-benchmarks --bench regression_bench

# All benchmarks
cargo bench -p bonsai-adaptive-benchmarks
```

## Integration with Existing Code

### 1. Integrate with InferenceEngine

```rust
// In bonsai-inference/src/lib.rs
use bonsai_adaptive_benchmarks::{
    PerformanceBenchmark, BenchmarkConfig, 
    MetricsCollector, BenchmarkLogger
};

impl InferenceEngine {
    pub fn new_with_benchmarking(
        model_registry: Arc<ModelRegistry>,
        kv_cache: Arc<KVCacheStore>,
        tool_registry: Arc<ToolRegistry>,
        tool_executor: Arc<ToolExecutor>,
        metrics: Arc<MetricsCollector>,
        logger: Arc<BenchmarkLogger>,
    ) -> Self {
        Self {
            model_registry,
            gpu_manager: GpuManager::new(),
            kv_cache,
            tool_registry,
            tool_executor,
            active_model: std::sync::RwLock::new(None),
            metrics,  // NEW: for recording inference metrics
            logger,   // NEW: for logging operations
        }
    }

    pub async fn generate_with_metrics(
        &self, 
        request: &InferenceRequest
    ) -> Result<GenerationOutput> {
        let start = std::time::Instant::now();
        
        let output = self.generate(request)?;
        
        // Record metrics
        let elapsed = start.elapsed().as_millis() as f32;
        self.metrics.record_latency(elapsed, 0, 1);
        
        Ok(output)
    }
}
```

### 2. Integrate with SystemEventBus

```rust
// In bonsai-workspace/src-tauri/src/system_event_bus.rs
use bonsai_adaptive_benchmarks::{
    RegressionDetector, RollbackManager,
    MetricsCollector
};

pub struct SystemEventBus {
    // ... existing fields
    regression_detector: RegressionDetector,
    rollback_manager: RollbackManager,
    metrics_collector: Arc<MetricsCollector>,
}

impl SystemEventBus {
    pub async fn on_model_change(&self, event: ModelChangeEvent) -> Result<()> {
        // Record baseline
        let baseline_metrics = self.collect_baseline_metrics().await?;
        
        // Run benchmarks on new model
        let current_metrics = self.collect_current_metrics().await?;
        
        // Check for regressions
        let report = self.regression_detector
            .detect_all_regressions(&baseline_metrics, &current_metrics);
        
        if self.rollback_manager.should_rollback(&report) {
            self.rollback_manager.perform_rollback(
                &report.baseline_version
            ).await?;
            
            self.emit_event(Event::RollbackPerformed {
                reason: format!("Regressions detected: {:?}", report.regressions_detected),
                previous_version: report.baseline_version,
            });
        }
        
        Ok(())
    }
}
```

### 3. Integrate with CI/CD Pipeline

Add to `.github/workflows/benchmark.yml`:

```yaml
name: Benchmark & Regression Detection

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

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
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/output.json
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true

  regression-detection:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: |
          cargo test -p bonsai-adaptive-benchmarks regression::
      - name: Run regression detection
        if: success()
        run: |
          cargo test -p bonsai-adaptive-benchmarks regression_bench --release
```

### 4. Add Metrics Recording to Key Paths

#### Scaling Operations
```rust
// In adaptive transformer scaling code
async fn scale_to_target(
    &mut self, 
    target_scale: u32,
    metrics: Arc<MetricsCollector>,
) -> Result<()> {
    let start = Instant::now();
    
    // ... perform scaling
    
    let elapsed = start.elapsed().as_secs_f32() * 1000.0;
    metrics.record("scaling_latency_ms", elapsed, target_scale, HashMap::new());
    
    Ok(())
}
```

#### Model Loading
```rust
async fn load_model_with_metrics(
    &self,
    name: &str,
    version: &str,
    metrics: Arc<MetricsCollector>,
) -> Result<()> {
    let start = Instant::now();
    
    self.load_model(name, version).await?;
    
    let elapsed = start.elapsed().as_secs_f32() * 1000.0;
    metrics.record("model_load_latency_ms", elapsed, 0, HashMap::new());
    
    Ok(())
}
```

#### Inference
```rust
pub async fn generate_with_metrics(
    &self,
    request: &InferenceRequest,
    metrics: Arc<MetricsCollector>,
) -> Result<GenerationOutput> {
    let start = Instant::now();
    
    let output = self.generate(request)?;
    
    let elapsed = start.elapsed().as_secs_f32() * 1000.0;
    metrics.record_latency(elapsed, 0, request.batch_size.unwrap_or(1));
    
    Ok(output)
}
```

## Configuration

### Custom Benchmark Config
```rust
let config = BenchmarkConfig {
    name: "my_benchmark".to_string(),
    description: "Custom benchmark".to_string(),
    scales: vec![100_000_000, 1_000_000_000],
    batch_sizes: vec![1, 8],
    sequence_lengths: vec![512],
    devices: vec!["gpu".to_string()],
    num_runs: 10,
    warmup_runs: 3,
    output_dir: PathBuf::from("./my_results"),
    ..Default::default()
};

let mut runner = BenchmarkRunner::new(config);
runner.run_all().await?;
```

### Custom Test Fixtures
```rust
let mut fixture = TestFixture::new("custom_fixture".to_string(), 42);
fixture.datasets.push(TestDataset::small_general());
fixture.datasets.push(TestDataset::medium_code());

for dataset in &fixture.datasets {
    let results = run_on_dataset(dataset).await?;
    // Process results
}
```

## Monitoring & Observability

### Real-time Metrics
```rust
let metrics = Arc::new(MetricsCollector::new());
let logger = Arc::new(BenchmarkLogger::new());

// Record as inference happens
metrics.record_latency(15.5, 100_000_000, 1);
metrics.record_memory(2048.0, 100_000_000);
metrics.record_perplexity(25.3, 100_000_000);

// Log events
let mut context = HashMap::new();
context.insert("scale".to_string(), "100M".to_string());
logger.info("Scaling operation started", context);

// Build dashboard
let dashboard = DashboardBuilder::new(metrics, logger).build();
println!("{}", serde_json::to_string_pretty(&dashboard)?);
```

### Export Metrics
```rust
let metrics = metrics_collector.aggregate_metrics();
let json = serde_json::to_string_pretty(&metrics)?;
std::fs::write("metrics.json", json)?;

let logs = benchmark_logger.export_logs();
std::fs::write("logs.json", logs)?;
```

## Automated Testing in Pipelines

### Pre-commit Hook
```bash
#!/bin/bash
# .git/hooks/pre-commit
cargo test -p bonsai-adaptive-benchmarks --lib
if [ $? -ne 0 ]; then
    echo "Unit tests failed!"
    exit 1
fi

cargo clippy -p bonsai-adaptive-benchmarks -- -D warnings
if [ $? -ne 0 ]; then
    echo "Clippy checks failed!"
    exit 1
fi
```

### Nightly Regression Detection
Create `scripts/nightly_regression_check.sh`:

```bash
#!/bin/bash
# Run comprehensive regression detection
cargo bench -p bonsai-adaptive-benchmarks --bench performance_bench --release

# Compare against baseline
BASELINE="baselines/v0.1.0.json"
CURRENT="target/criterion/latest.json"

# Check regressions (using custom script)
python3 scripts/check_regressions.py --baseline "$BASELINE" --current "$CURRENT"

if [ $? -ne 0 ]; then
    echo "Regressions detected! Check ./regression_report.json"
    exit 1
fi
```

## Troubleshooting

### Issue: Tests Timeout
**Solution**: Increase test timeout or reduce sample count
```toml
[profile.test]
opt-level = 2
```

### Issue: Benchmark Variance Too High
**Solution**: Increase warmup runs or number of samples
```rust
let config = BenchmarkConfig {
    warmup_runs: 5,
    num_runs: 20,
    ..Default::default()
};
```

### Issue: Memory Leak Detected
**Solution**: Use `valgrind` or `heaptrack`
```bash
valgrind cargo test -p bonsai-adaptive-benchmarks --lib
```

### Issue: Regression Threshold Too Strict
**Solution**: Adjust regression threshold
```rust
let detector = RegressionDetector::new(10.0);  // 10% instead of 5%
```

## Performance Tips

1. **Run Benchmarks in Release Mode**:
   ```bash
   cargo bench -p bonsai-adaptive-benchmarks --release
   ```

2. **Reduce System Noise**:
   - Close unnecessary applications
   - Disable power management
   - Use isolated CPUs if available

3. **Use Profiling Tools**:
   ```bash
   cargo bench -p bonsai-adaptive-benchmarks --bench performance_bench -- --profile-time=10
   ```

4. **Cache Baseline Locally**:
   ```bash
   cargo bench -p bonsai-adaptive-benchmarks -- --save-baseline my_baseline
   cargo bench -p bonsai-adaptive-benchmarks -- --baseline my_baseline
   ```

## Next Steps

1. **Integrate with InferenceEngine**: Add metrics recording to inference paths
2. **Set up CI/CD**: Add benchmark jobs to GitHub Actions
3. **Configure Regression Detection**: Set thresholds for your use case
4. **Monitor in Production**: Stream metrics to dashboard
5. **Automate Rollback**: Trigger rollback on regression detection
6. **Custom Benchmarks**: Add domain-specific tests
7. **Cost Optimization**: Track and optimize inference costs per scale

## Support

For issues or feature requests, create an issue in the main repository with:
- Benchmark type (unit, integration, regression)
- Scale configuration tested
- Error message or unexpected behavior
- CPU/GPU specs
