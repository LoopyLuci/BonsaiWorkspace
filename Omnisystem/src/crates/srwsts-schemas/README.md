# SRWSTS Schemas

YAML schema definitions, parsing, and validation for SRWSTS test plans.

Converts human-readable YAML test specifications into strongly-typed Rust structures with comprehensive validation.

## Features

- YAML schema parsing with serde_yaml
- Automatic conversion to srwsts-core types
- Comprehensive validation of test plans
- Resource limit enforcement
- Fault parameter validation
- Support for custom fault types and parameters

## YAML Schema

### Top-Level Structure

```yaml
version: "1.0"
metadata:
  id: "unique-test-id"
  name: "Human Readable Name"
  description: "What this test validates"
  plan_version: "1.0"  # Optional
  tags:               # Optional
    - kernel
    - stress

resource_limits:
  max_cpu_percent: 100
  max_memory_bytes: 4294967296
  max_disk_bytes: 107374182400
  max_file_descriptors: 65536
  max_threads: 1024
  max_network_mbps: 1000

workloads:
  - id: "w1"
    type: "cpu_stress"
    concurrency: 16
    duration_secs: 300
    ops_per_sec: 0
    params:
      cpu_cores: "8"

faults:
  - id: "f1"
    type: "cpu_stress"
    inject_at_secs: 60
    duration_secs: 120
    enabled: true
    params:
      cpu_count: 8

max_duration_secs: 3600
```

## Supported Workload Types

- `cpu_stress`: CPU-intensive work
- `disk_io_stress`: Disk I/O operations
- `memory_stress`: Memory allocation and access
- `http_request_stress`: HTTP request generation
- `background_task`: Background work
- `file_io_stress`: File operations

## Supported Fault Types

- `cpu_stress`: CPU overload
- `memory_exhaustion`: Memory pressure
- `disk_io_stress`: I/O bottleneck
- `network_packet_loss`: Packet loss simulation
- `network_latency`: Network delay
- `kernel_panic`: System panic
- `task_scheduling_stress`: Context switch stress
- `file_descriptor_exhaustion`: FD limit exhaustion
- `process_termination`: Process SIGKILL
- `signal_injection`: Custom signals

## Validators

- **SchemaValidator**: Validates test plans against configurable limits
  - CPU percentage (0-100)
  - Memory limits
  - Thread limits
  - Workload concurrency
  - Fault timing vs test duration
  - Fault-specific parameters

## Usage

```rust
use srwsts_schemas::*;

// Parse YAML string
let yaml = std::fs::read_to_string("test.yaml")?;
let plan = parse_test_plan(&yaml)?;

// Or use parser directly
let parser = TestPlanParser::new();
let plan = parser.parse_file(std::path::Path::new("test.yaml"))?;

// Validate with custom limits
let validator = SchemaValidator::new()
    .with_max_concurrency(10000)
    .with_max_duration(86400);
validator.validate(&plan)?;
```

## Examples

See `examples/` directory for complete test plans:
- `kernel-scheduler-heavy-load.yaml` - Kernel scheduler stress test
- `service-stress.yaml` - Service under heavy load with faults
- `file-descriptor-exhaustion.yaml` - FD limit testing

## Testing

Run all tests:
```bash
cargo test --lib -p srwsts-schemas
```

All 15 tests pass covering parsing, validation, and conversion.
