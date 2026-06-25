# SRWSTS Applications - Quick Start Guide

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
srwsts-applications = { path = "../srwsts-applications" }
```

## Running Tests

```bash
# Run all stress tests
cargo test -p srwsts-applications

# Run specific test
cargo test -p srwsts-applications test_concurrent_file_editing

# Run with output
cargo test -p srwsts-applications -- --nocapture
```

## Basic Example

```rust
use srwsts_applications::{
    ApplicationStressConfig, ApplicationStressEnvironment, ApplicationTestRunner, TestContext,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure
    let config = ApplicationStressConfig::default();
    
    // Create environment
    let env = ApplicationStressEnvironment::new(config).await?;
    env.initialize().await?;
    
    // Create runner
    let runner = ApplicationTestRunner::new();
    
    // Create context
    let ctx = TestContext::new(
        "run-1", "test-1",
        env.metrics.clone(),
        PathBuf::from("./artifacts"),
        600
    );
    
    // Run tests
    let results = runner.run_all(&ctx).await?;
    
    // Check results
    let agg = ApplicationTestRunner::aggregate_results(&results);
    println!("Pass rate: {:.2}%", agg.pass_rate());
    
    env.shutdown().await?;
    Ok(())
}
```

## Key Components

### ApplicationBootstrap
```rust
let bootstrap = ApplicationBootstrap::new().await?;
bootstrap.bootstrap().await?;

// Check health
let healthy = bootstrap.is_healthy().await;
println!("{}", bootstrap.status().await);
```

### Stress Tests
```rust
// Run Workspace tests
let result = WorkspaceStressTest::test_concurrent_file_editing(&ctx).await?;

// Run Buddy tests
let result = BuddyStressTest::test_offline_online_transitions(&ctx).await?;

// Run OmniBot tests
let result = OmniBotStressTest::test_concurrent_chat_sessions(&ctx).await?;
```

### Fault Injection
```rust
use srwsts_applications::{FaultScenario, FaultScenarioExecutor};

let scenario = FaultScenario::app_crash("workspace");
let result = FaultScenarioExecutor::execute(&scenario).await?;
println!("Recovery time: {}ms", result.recovery_time_ms.unwrap());
```

### User Simulation
```rust
use srwsts_applications::{InputSimulator, UserSimulation};

let simulator = InputSimulator::new();
let user = UserSimulation::new("user-1", 3600, 10.0);
user.simulate_session(&simulator).await?;
```

### Metrics
```rust
// Record operations
ctx.metrics.record_response_time("query", Duration::from_millis(50));
ctx.metrics.record_memory("heap", 1024 * 1024 * 512);

// Get summary
let summary = ctx.metrics.summary();
println!("Peak memory: {} MB", summary.memory.peak_mb);
```

## Configuration

```rust
use srwsts_applications::ApplicationStressConfig;

let config = ApplicationStressConfig {
    max_concurrent_apps: 10,
    max_concurrent_users: 100,
    test_timeout_secs: 600,
    verbose: true,
    store_artifacts: true,
    artifact_dir: PathBuf::from("./artifacts"),
    profile_performance: true,
    enable_fault_injection: true,
    memory_monitor_interval_ms: 500,
    deterministic: false,
};

let env = ApplicationStressEnvironment::new(config).await?;
```

## Test Categories

### Workspace (5 tests)
- Concurrent file editing (500 files)
- Continuous compilation
- Developer workday simulation
- Multi-user collaboration (50 users)
- Memory leak detection

### Buddy (5 tests)
- Offline-online transitions (100x)
- CRDT merge stress (1,000 updates)
- Large file sync (1GB+)
- AI query throughput (1,000 concurrent)
- Snapshot recovery

### OmniBot (5 tests)
- Concurrent chat (1,000 sessions)
- NLP parsing accuracy
- Task execution (10,000 concurrent)
- Memory-constrained AI
- Network recovery

## Fault Scenarios

```rust
FaultScenario::app_crash("workspace")
FaultScenario::network_loss("buddy")
FaultScenario::storage_corruption("workspace")
FaultScenario::gpu_reset()
FaultScenario::memory_exhaustion("omnibot")
```

## Interaction Scenarios

```rust
InteractionScenario::workspace_buddy_sync()
InteractionScenario::buddy_omnibot_query()
InteractionScenario::fullstack_integration()
InteractionScenario::cascading_failure()
```

## Common Operations

### Save Test Artifact
```rust
ctx.save_artifact("test.txt", b"content").await?;
```

### Check Artifact Exists
```rust
let exists = ctx.artifact_exists("test.txt").await?;
```

### Load Artifact
```rust
let data = ctx.load_artifact("test.txt").await?;
```

### Record Metrics
```rust
ctx.metrics.record_response_time("op", Duration::from_millis(100));
ctx.metrics.record_memory("heap", 512 * 1024 * 1024);
ctx.metrics.record_compilation(Duration::from_millis(5000));
```

## Performance Tips

1. Use `deterministic: true` for reproducible results
2. Set `memory_monitor_interval_ms: 1000` for lower overhead
3. Enable `profile_performance` only for bottleneck analysis
4. Use appropriate `test_timeout_secs` per test type
5. Adjust `max_concurrent_*` based on system resources

## Troubleshooting

### Tests Timeout
- Increase `test_timeout_secs`
- Reduce `max_concurrent_users`
- Check system resources

### Memory Issues
- Reduce `max_concurrent_users`
- Set shorter `memory_monitor_interval_ms`
- Disable artifact storage

### High CPU Usage
- Reduce concurrency settings
- Increase `think_time_ms` in user simulations
- Use deterministic mode

## Next Steps

- Read SRWSTS_APPLICATIONS.md for architecture details
- Check examples/stress_test_suite.rs for complete example
- Review test implementations in src/tests/
- Run benchmarks with `cargo bench -p srwsts-applications`
