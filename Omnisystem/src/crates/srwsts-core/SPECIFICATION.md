# SRWSTS Core Specification

**Version:** 0.1.0  
**Status:** Production-Ready  

## Overview

SRWSTS (Stress, Resilience, and Workload System Test Suite) Core provides the foundational type system and traits for comprehensive system stress testing and resilience validation.

## Core Types

### Identifiers

#### RunId
Unique identifier for a test execution run. Internally uses UUID v4.

#### TestId
Named identifier for a test case.

### Time & Duration

#### Duration
High-precision duration with nanosecond accuracy.

Methods: `from_secs()`, `from_millis()`, `as_millis()`, `as_secs_f64()`, `add()`, `sub()`, `mul_f64()`, `is_zero()`

#### Timestamp
Unix epoch timestamp with nanosecond precision.

Methods: `now()`, `from_secs()`, `to_system_time()`, `elapsed()`, `add_duration()`, `sub_duration()`

### Test Specification

#### Workload
Definition of work to be performed.

**Fields:**
- `id`: Unique identifier
- `workload_type`: Type of work (cpu_stress, memory_stress, etc.)
- `concurrency`: Number of concurrent tasks
- `duration_secs`: How long to run
- `ops_per_sec`: Target throughput (0 = unlimited)
- `parameters`: Custom workload-specific parameters

**Validation:**
- Concurrency must be > 0
- Duration must be > 0

#### FaultDefinition
Specification of a fault to inject.

**Fields:**
- `id`: Unique identifier
- `fault_type`: Type of fault (CPU stress, memory exhaustion, etc.)
- `inject_at_secs`: When to start injection
- `duration_secs`: How long the fault lasts
- `enabled`: Whether to inject this fault
- `parameters`: Fault-specific parameters

**Supported Fault Types:**
- CpuStress
- MemoryExhaustion
- DiskIoStress
- NetworkPacketLoss
- NetworkLatency
- KernelPanic
- TaskSchedulingStress
- FileDescriptorExhaustion
- ProcessTermination
- SignalInjection
- Custom(String)

#### ResourceLimits
Resource constraints for test execution.

**Fields:**
- `max_cpu_percent`: 0.0-100.0
- `max_memory_bytes`: Default 4GB
- `max_disk_bytes`: Default 100GB
- `max_file_descriptors`: Default 65536
- `max_threads`: Default 1024
- `max_network_mbps`: Default 1000

#### TestPlan
Complete test specification.

**Fields:**
- `id`: Unique identifier
- `name`: Human-readable name
- `description`: What this test validates
- `version`: Plan version
- `workloads`: Vec<Workload>
- `faults`: Vec<FaultDefinition>
- `resource_limits`: ResourceLimits
- `max_duration_secs`: Maximum test duration
- `metadata`: Custom key-value metadata

**Validation:**
- Must have at least one workload
- All workloads must be valid
- All faults must be valid
- max_duration >= longest workload duration

### Results & Metrics

#### AssertionResult
Single assertion outcome.

**Fields:**
- `id`: Assertion identifier
- `description`: What was asserted
- `passed`: Whether assertion passed
- `expected`: Expected value (optional)
- `actual`: Actual value (optional)
- `error_message`: Error details if failed

#### FaultEvent
Record of a fault injection event.

**Fields:**
- `fault_id`: Which fault was injected
- `injected_at`: Timestamp when injected
- `recovered`: Whether system recovered
- `recovered_at`: When recovery was detected
- `system_state`: Captured system state
- `recovery_actions`: Actions taken to recover

#### TestResult
Complete test execution result.

**Fields:**
- `result_id`: UUID of this result
- `test_id`: Which test ran
- `run_id`: Which run this is
- `started_at`: Execution start time
- `completed_at`: Execution end time
- `status`: ResultStatus (Pass, Fail, Error, etc.)
- `assertions`: Vec<AssertionResult>
- `fault_events`: Vec<FaultEvent>
- `metrics`: TestMetrics
- `stdout`/`stderr`: Captured output
- `execution_error`: Any error that occurred
- `custom_data`: Custom metadata

### Status Enums

#### ExecutionStatus
- Pending, Running, Passed, Failed, Cancelled, Error, Timeout, Skipped

#### ResultStatus
- Pass, Fail, Error, Timeout, Cancelled, NotRun

#### FaultOutcome
- Injected, Recovered, NotRecovered, Skipped

## Traits

### TestExecutor
Trait for executing tests. Implementers can run test plans and return results.

### FaultInjector
Trait for injecting faults. Implementers inject various fault types and track recovery.

### ResultCollector
Trait for collecting and storing test results.

### TestHook
Trait for test lifecycle callbacks (before_test, after_test, etc.).

### SystemMonitor
Trait for monitoring system behavior during tests.

## Error Handling

### SrwstsError

Comprehensive error enum with 40+ variants covering all failure scenarios.

**Categories:**
- Initialization errors
- Test execution errors
- Fault injection errors
- Resource limit errors
- Result collection errors
- Metrics validation errors
- Concurrency errors
- IO errors
- State machine errors
- External integration errors

All errors include:
- Clear error messages
- Context information
- Recovery suggestions via `recovery_suggestion()`

## Configuration

```rust
pub struct SrwstsConfig {
    pub max_concurrent: usize,
    pub test_timeout: std::time::Duration,
    pub enable_faults: bool,
    pub enable_collection: bool,
    pub deterministic: bool,
}
```

## Testing

- 53 unit tests in srwsts-core
- 100% passing rate
- Comprehensive coverage of all types and methods

## Serialization

All types implement `Serialize` and `Deserialize` for JSON/YAML persistence via serde.

## Thread Safety

All types implement `Send + Sync`:
- Safe for concurrent access
- Uses Arc<RwLock<T>> for shared state
- DashMap for concurrent collections
- Proper synchronization primitives
