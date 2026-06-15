# SRWSTS Core

Core infrastructure for the Stress, Resilience, and Workload System Test Suite (SRWSTS).

Provides fundamental types, traits, and error handling for stress testing Bonsai Ecosystem components.

## Core Types

- **TestPlan**: Complete specification of a test, including workloads and faults
- **Workload**: Definition of work to be performed (CPU stress, disk I/O, etc.)
- **FaultDefinition**: Specification of faults to inject (CPU stress, memory exhaustion, etc.)
- **TestResult**: Complete execution results including assertions and metrics
- **ResourceMetrics**: Resource usage (CPU, memory, threads, I/O)
- **TestMetrics**: Comprehensive metrics from test execution
- **RunId/TestId**: Unique identifiers for test runs and cases

## Key Traits

- **TestExecutor**: Executes test plans
- **FaultInjector**: Injects faults into the system
- **ResultCollector**: Collects and stores test results
- **SystemMonitor**: Monitors system metrics during tests
- **TestHook**: Callbacks for test events

## Error Handling

Comprehensive `SrwstsError` type covering:
- Initialization failures
- Test execution errors
- Fault injection failures
- Resource limit violations
- Result collection errors
- Metrics validation errors

All errors include recovery suggestions.

## Status Types

- **ExecutionStatus**: Pending, Running, Passed, Failed, Cancelled, Error, Timeout, Skipped
- **ResultStatus**: Pass, Fail, Error, Timeout, Cancelled, NotRun
- **FaultOutcome**: Injected, Recovered, NotRecovered, Skipped

## Example

```rust
use srwsts_core::*;

// Create a test plan
let plan = TestPlan::new("test1", "My Test", "Description")
    .with_workload(
        Workload::new("w1", "cpu_stress", 4, 300)
    )
    .with_fault(
        FaultDefinition::new("f1", FaultType::CpuStress, 60, 120)
    );

// Validate the plan
plan.validate()?;

// Create a test result
let result = TestResult::new(
    TestId::new("test1"),
    RunId::new(),
    Timestamp::now(),
);

// Collect results
let result = result
    .add_assertion(AssertionResult::new("a1", "check passed", true))
    .mark_passed();
```

## Testing

Run all tests:
```bash
cargo test --lib -p srwsts-core
```

All 50+ tests pass with comprehensive coverage of types, validation, and error handling.
