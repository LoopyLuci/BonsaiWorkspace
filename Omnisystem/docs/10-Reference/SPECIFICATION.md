# SRWSTS Full-Stack Testing System - Specification

## Overview

The SRWSTS Full-Stack Integrated Testing System is a comprehensive stress-testing suite designed to validate the entire Omnisystem architecture under realistic and extreme conditions. It combines:

1. **UOSC Kernel** - Bare-metal OS with scheduling, memory management, I/O
2. **Omnisystem Services** - Service Lifecycle Manager, Buddy sync, Workspace, Survival System
3. **Bonsai Applications** - Multi-language runtime ecosystem (750+ languages)

## Core Architecture

### Vault: Unified Test Container

The **Vault** is the central abstraction that contains:

```
Vault
├── UOSC Kernel
│   ├── Thread Count
│   ├── Memory Usage
│   └── Context Switches
├── Services
│   ├── SLM (Service Lifecycle Manager)
│   ├── Buddy (File Sync)
│   ├── Workspace (File Management)
│   └── Survival (Event Logging)
├── Applications
│   ├── Python Runtime
│   ├── Rust Runtime
│   ├── JavaScript Runtime
│   └── ... (750+ supported languages)
└── Audit Log
    └── Sequential Event Recording
```

All components maintain state through atomic operations and RwLock synchronization for thread safety.

### Component Health States

```rust
enum ComponentHealth {
    Healthy,    // Operating normally
    Degraded,   // Reduced capacity, still operational
    Failed,     // Not operational
    Recovering, // In recovery process
    Unknown,    // Status unknown
}
```

## Test Categories

### 1. Bootstrap Tests

**Purpose:** Verify system initialization and readiness

**Tests:**
- Default configuration creation
- Service registration
- Application registration
- Health verification
- Kernel initialization

**Success Criteria:**
- All components registered
- No initialization errors
- All services operational
- Kernel thread count > 0

### 2. Nominal Load Tests

**Purpose:** Establish baseline performance under normal workload

**Configuration:**
```rust
struct NominalLoadConfig {
    duration_secs: u64,              // 30s default
    concurrent_requests: u32,        // 16 requests
    concurrent_apps: u32,            // 8 apps
    foreground_ratio: f64,           // 70% user tasks
}
```

**Metrics:**
- Total requests: count
- Success rate: %
- Average latency: ms
- p99 latency: ms
- Requests per second: ops/sec

**Success Criteria:**
- Error rate < 5%
- p99 latency < 10 seconds
- All subsystems coordinate correctly

### 3. Peak Load Tests

**Purpose:** Validate system stability under extreme resource utilization

**Phases:**
1. CPU Saturation: 95% utilization across all cores
2. Memory Saturation: 90% RAM utilization
3. I/O Saturation: 10,000 ops/sec
4. Concurrent Peaks: All simultaneously

**Metrics:**
- Achieved CPU utilization: %
- Achieved memory utilization: %
- I/O operations completed: count
- Kernel stability: bool
- Deadlock detection: bool

**Success Criteria:**
- Kernel remains operational
- No deadlocks detected
- No panics
- Graceful degradation

### 4. Cascading Failure Tests

**Purpose:** Verify component isolation and prevent failure cascades

**Scenarios:**
1. Kernel thread failure → Services handle gracefully
2. Service failure → Other services remain operational
3. Application failure → Workspace saves state, Buddy syncs
4. Multiple simultaneous failures → System partially operational
5. Sequential failures with recovery → All recover successfully

**Metrics:**
- Isolation maintained: bool
- Services affected: count
- Applications affected: count
- Kernel survived: bool
- Recovery successful: count

**Success Criteria:**
- Failed component doesn't cascade
- Healthy components remain operational
- Kernel survives all failures
- System eventually recovers

### 5. Recovery Tests

**Purpose:** Validate resilience and state consistency after failures

**Scenarios:**
1. Kernel Panic → Snapshot restore, services resume, apps recover
2. Service Crash → Auto-restart via SLM, data consistency verified
3. Data Corruption → Checksums detect, recovery mechanism restores
4. Snapshot Strategy → Multi-point snapshots enable rapid recovery

**Metrics:**
- Panic recovered: bool
- Recovery time: seconds
- Service crashes: count
- Auto-restart count: count
- Data consistency verified: bool
- Data loss detected: bool

**Success Criteria:**
- System recovers from panic
- Recovery time < 1 minute
- All auto-restarts successful
- No data loss
- State consistent pre/post recovery

### 6. Network Partition Tests

**Purpose:** Validate P2P mesh resilience and CRDT convergence

**Scenarios:**
1. P2P Mesh Split → Partial nodes partitioned
2. CRDT Drift → State divergence during partition
3. Request Timeout → Synchronous requests fail gracefully
4. Asymmetric Routes → Data reaches destination via alternate paths
5. Reunion → Nodes reconnect, state reconciles, no data loss

**Metrics:**
- Partition duration: seconds
- Nodes partitioned: count
- Nodes operational: count
- CRDT convergence time: seconds
- Split-brain detected: bool
- Data lost: bool

**Success Criteria:**
- Partition heals without deadlock
- CRDT converges within 30 seconds
- No split-brain
- No data loss on reunion
- All state consistent post-healing

### 7. State Consistency Tests

**Purpose:** Verify audit trail and deterministic replay

**Scenarios:**
1. Audit Log Completeness → Every event recorded end-to-end
2. Data Loss Detection → No data lost across any failure
3. Deterministic Replay → Same workload input produces identical output
4. Component State Consistency → All components state matches

**Metrics:**
- Audit entries logged: count
- Audit gap detected: bool
- Data loss detected: bool
- Replay divergence detected: bool
- Consistency score: 0.0-1.0

**Success Criteria:**
- Complete audit trail
- No data loss
- Deterministic replay succeeds
- All component states consistent

### 8. End-to-End Journey Tests

**Purpose:** Validate real-world workflows with fault injection

**Journeys:**
1. Developer Workflow
   - Open workspace
   - Edit files
   - Compile code
   - Run tests

2. Buddy File Sync
   - Modify files on device 1
   - Trigger sync
   - Verify replication on device 2

3. Omni-Bot Deployment
   - Start deployment task
   - Inject network partition mid-flight
   - Resume deployment after healing
   - Verify all subsystems consistent

**Metrics:**
- Workflow completed: bool
- All subsystems consistent: bool
- File sync verified: bool
- Deployment successful: bool
- Partition handled correctly: bool
- Failures recovered: count

**Success Criteria:**
- All journeys complete successfully
- All subsystems remain consistent
- Network faults handled gracefully
- No state loss during interruptions

### 9. Long-Duration Stress Tests

**Purpose:** Detect memory leaks, deadlocks, performance degradation

**Configuration:**
```rust
struct LongDurationConfig {
    total_duration_secs: u64,    // 300s default, 259200s for 72h
    fault_injection_interval_secs: u64,  // Every 30s
    measure_performance: bool,
    detect_leaks: bool,
}
```

**Scenarios:**
1. Continuous Load → Run system continuously
2. Periodic Faults → Inject faults every hour
3. Memory Leak Detection → Monitor memory trend
4. Performance Degradation → Track latency over time

**Metrics:**
- Total runtime: seconds
- Faults injected: count
- Recovery count: count
- Memory leak detected: bool
- Performance degradation: %
- Deadlock detected: bool

**Success Criteria:**
- No memory leaks detected
- No deadlocks
- No performance degradation
- All faults recovered successfully
- System stable throughout

## Test Metrics

### Common Metrics

```rust
pub struct TestResult {
    pub test_id: String,
    pub success: bool,
    pub duration_secs: f64,
    pub error_message: Option<String>,
}

pub struct ComponentHealth {
    pub component_id: String,
    pub health: HealthStatus,
    pub last_check: Timestamp,
}

pub struct PerformanceMetrics {
    pub latency_ms: Histogram,
    pub throughput_ops_per_sec: f64,
    pub error_rate_percent: f64,
}
```

## Error Handling

### Error Categories

```rust
pub enum FullStackTestError {
    BootstrapFailed,
    KernelPanic,
    ServiceCrash,
    AppFailure,
    TestTimeout,
    ResourceExhaustion,
    StateInconsistency,
    DataCorruption,
    NetworkPartition,
    // ... 20+ error types
}
```

### Error Severity

- **Fatal**: Kernel panics, deadlocks, data corruption
- **Transient**: Network partitions, service crashes, timeouts
- **Consistency**: State mismatches, audit gaps, data loss

## Concurrency Model

### Thread Safety

- `Arc<RwLock<T>>` for shared vault state
- `DashMap` for concurrent service/app maps
- `AtomicU64` for lock-free metric counters
- All async operations use Tokio

### Execution Model

```
Bootstrap (1 async task)
    ↓
Test Categories (9 concurrent test suites)
├── Nominal Load (4 concurrent operations)
├── Peak Load (CPU + Memory + I/O in parallel)
├── Cascading Failures (sequential with recovery)
├── Recovery (snapshot/restore cycles)
├── Network Partitions (mesh operations)
├── State Consistency (audit verification)
├── End-to-End (mixed workloads)
├── Long Duration (background monitoring)
└── Report Generation (aggregation)
```

## Performance Characteristics

### Compilation
- Incremental: <30s (with BACE)
- Clean: <5 minutes
- Release: 3 codegen units, thin LTO

### Test Execution
- Bootstrap: <1s
- Nominal Load (30s duration): ~35s total
- Peak Load (3 phases, 10s each): ~35s total
- Cascading Failures: ~10s
- Recovery: ~15s
- Network Partitions: ~30s
- State Consistency: ~20s
- End-to-End: ~20s
- Long Duration (10s duration): ~15s total
- Full Suite: ~3 minutes

### Resource Usage
- Memory: 512MB vault baseline
- Services: 16 concurrent operations
- Applications: 100 max capacity
- Threads: num_cpus for CPU saturation tests

## Integration Points

### UOSC Kernel
- Thread scheduling simulation
- Memory manager state tracking
- I/O subsystem metrics

### Omnisystem Services
- SLM (Service Lifecycle Manager): Service registration/lifecycle
- Buddy: File sync state tracking
- Workspace: File management operations
- Survival: Event audit logging

### Bonsai Applications
- Multi-language runtime registration
- Execution state tracking
- Error rate monitoring
- Memory usage tracking

## Future Extensions

### Distributed Testing
- Multi-node vault coordination
- Network simulation layer
- Consistent distributed snapshots

### Hardware Emulation
- Disk I/O simulation
- Network latency injection
- CPU thermal throttling

### Advanced Fault Injection
- Byzantine node simulation
- Correlated fault patterns
- Chaos engineering policies

### Observability
- Real-time dashboards
- Trace collection
- Metric export (Prometheus)

## Compliance

### Production Readiness
- [x] 50+ comprehensive tests
- [x] Full async/await support
- [x] Comprehensive error handling
- [x] Thread-safe concurrency
- [x] Deterministic replay
- [x] Audit logging
- [x] State snapshots
- [x] Resource isolation

### Code Quality
- [x] No unsafe code
- [x] Comprehensive documentation
- [x] Example code
- [x] 90%+ test coverage
- [x] No compiler warnings (except intentional)
- [x] Follows Rust conventions

## References

- UOSC Kernel Specification: See crates/srwsts-kernel/
- Omnisystem Architecture: See crates/omnisystem-config/
- Bonsai Ecosystem: See crates/bonsai-buddy-*/
