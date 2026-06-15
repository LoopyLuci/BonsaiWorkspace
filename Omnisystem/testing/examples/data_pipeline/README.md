# Data Pipeline Example

This example demonstrates Omnisystem's Sylva language orchestrating data transformations with Titan compute kernels and comprehensive telemetry.

## What This Shows

- **Sylva scripting**: Data loading, transformation, filtering
- **Titan integration**: Call vectorized operations from Sylva
- **Loops and conditionals**: Basic control flow
- **Array operations**: Indexing, slicing, aggregation
- **Telemetry tracking**: Every operation is traced for visualization
- **Time-travel debugging**: Inspect, rewind, and replay computations

## Architecture

```
Sylva (Script/REPL)
├── Load data (CSV, JSON, or hardcoded)
├── Call Titan vectorized ops
│   └── add_vectors([1,2,3], [4,5,6]) → [5,7,9]
│   └── normalize(matrix)
│   └── matrix_multiply(A, B)
├── Transform pipeline
│   ├── Filter: rows where sum > 5
│   ├── Aggregate: compute row sums
│   └── Sort: by value or key
├── Emit telemetry
│   ├── Timer events for each operation
│   └── Data movement events
└── Export or visualize
    ├── JSON output file
    ├── Telemetry dashboard
    └── Time-travel trace
```

## Running

### In the REPL
```bash
$ build repl -f main.sy
```

Outputs:
```
Loaded data: 3x3 matrix
Row 1 + Row 2 = [5.0, 7.0, 9.0]
Result: [5.0, 7.0, 9.0]
Filtered 2 rows
Row sums: [6.0, 15.0]
Pipeline complete. Use :trace to inspect execution.
```

### With time-travel debugging
```bash
$ build repl
sylva> x = [1, 2, 3]
  = [1, 2, 3]
sylva> y = [4, 5, 6]
  = [4, 5, 6]
sylva> z = add_vectors(x, y)
  = [5, 7, 9]
sylva> :trace
  [0] x = [1, 2, 3]           = [1, 2, 3]
  [1] y = [4, 5, 6]           = [4, 5, 6]
  [2] z = add_vectors(x, y)   = [5, 7, 9]
sylva> :rewind 1
  Rewound to step 1. Environment restored.
sylva> y = [10, 20, 30]
  = [10, 20, 30]
sylva> :replay
  Replaying 1 step(s)...
  [2] z = add_vectors(x, y)
    = [11, 22, 33]
    Result changed from [5, 7, 9] (deterministic replay detected)
```

## Telemetry

Every operation in the pipeline is tracked:

```
Timer events:
  - data_load: 0.5ms
  - add_vectors: 0.05ms (per call)
  - filter: 1.2ms
  - aggregate: 0.8ms

Data movement:
  - Loaded 9 floats
  - Processed 6 rows
  - Output 2 rows
  - Total memory: 1.2 KB

Trust score: 82/100
  (unsigned code gets 60-70; higher with formal verification)
```

## Advanced: Integration with Aether

In Phase 3, the data pipeline could be distributed:

```
Sylva (orchestrator)
└── Spawn Aether actor per partition
    └── Actor 1: Process rows [0:3]
    └── Actor 2: Process rows [3:6]
    └── Actor 3: Process rows [6:9]
    └── Merge results with eventual consistency

Result: Parallel + fault-tolerant pipeline
```

## Extending This Example

1. **Real data**: Load from CSV with `build load`
2. **More Titan ops**: Call matrix_multiply, FFT, etc.
3. **Visualization**: Use `build observe` to see telemetry graph
4. **Export**: Save results to JSON/Parquet
5. **Stress test**: Scale to 1GB+ datasets

## See Also

- `hello_world.build` — Basic cross-language example
- `web_service/` — Aether actor example
- GETTING_STARTED.md — Time-travel debugging tutorial
- CHANGELOG.md — Roadmap for Phase 3+ features
