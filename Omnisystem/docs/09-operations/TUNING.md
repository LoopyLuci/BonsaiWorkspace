# Performance Tuning Guide

**Fine-tune Omnisystem for your workload**

---

## TITAN Tuning

### Compilation Flags
```bash
# Optimize for speed
omnisystem compile --optimize 3 --lto program.ti

# Optimize for size
omnisystem compile --optimize z program.ti

# Debug mode (slow, better for development)
omnisystem compile --debug program.ti
```

### Runtime Tuning
```bash
# Thread pool size
export OMNISYSTEM_THREADS=8

# Stack size
export OMNISYSTEM_STACK_SIZE=4m

# Allocator
export OMNISYSTEM_ALLOCATOR=mimalloc
```

### Memory Tuning
```bash
# GC frequency (0-100)
export GC_FREQUENCY=50

# Heap size
export HEAP_SIZE=2g
```

---

## SYLVA Tuning

### Tensor Operations
```sylva
// Use appropriate precision
let t32 = Tensor::<f32>::randn([1000, 1000])  // 4MB
let t64 = Tensor::<f64>::randn([1000, 1000])  // 8MB

// Reuse tensors to reduce allocation
let mut result = Tensor::zeros([100, 100])
for batch in loader {
    compute_into(&mut result, &batch)
}
```

### GPU Acceleration
```bash
# Enable GPU
export OMNISYSTEM_GPU=cuda

# GPU memory
export GPU_MEMORY=8g

# Batch size for GPU
export BATCH_SIZE=128
```

### Training Optimization
```sylva
// Mixed precision
let trainer = Trainer::new()
    .with_mixed_precision(true)  // FP32/FP16

// Gradient accumulation
trainer.with_accumulation_steps(4)

// Learning rate scheduling
let scheduler = CosineAnnealingLR::new(
    initial_lr: 0.1,
    min_lr: 0.0001,
    T_max: 100
)
```

---

## AETHER Tuning

### Consensus Tuning
```aether
let cluster = Cluster::new()
    .with_heartbeat_interval(150)        // Lower = faster detection
    .with_election_timeout(300..600)     // Wider = more stable
    .with_max_log_entries(10000)         // Tune for memory

// For fast networks: lower values
// For slow networks: higher values
```

### Replication Tuning
```aether
// Replication factor
.with_replication_factor(3)  // Minimum for safety

// Read mode
.with_read_consistency(ReadConsistency::Eventual)  // Faster reads
// vs
.with_read_consistency(ReadConsistency::Strong)    // Slower but consistent
```

### Batch Tuning
```aether
// Batch writes for throughput
let batch = WriteBatch::new()
    .put("key1", "val1")
    .put("key2", "val2")
    .put("key3", "val3")
    // ... 100+ operations

store.write_batch(batch)?
// 100x throughput improvement
```

---

## AXIOM Tuning

### Proof Search Tuning
```axiom
let prover = TheoremProver::new()
    .with_timeout(Duration::from_secs(10))
    .with_depth_limit(100)
    .with_memoization(true)

// Smaller depth for faster proofs
// Memoization trades memory for speed
```

### Type Inference Tuning
```axiom
let mut infer = TypeInference::new()
    .with_max_iterations(100)
    .with_constraint_propagation(true)

// Enable constraint propagation for harder problems
```

---

## JVM/Runtime Tuning

### Java/JVM Options
```bash
export JVM_OPTS="-Xmx2g -Xms1g -XX:+UseG1GC"
```

### GC Tuning
```bash
# Parallel GC (default)
export GC_TYPE=parallel

# G1GC (better for large heaps)
export GC_TYPE=g1

# ZGC (low latency)
export GC_TYPE=z
```

---

## Network Tuning

### TCP Settings
```bash
# Increase buffer sizes
sysctl -w net.core.rmem_max=134217728
sysctl -w net.core.wmem_max=134217728

# Increase backlog
sysctl -w net.core.somaxconn=4096
```

### Connection Pooling
```bash
# AETHER cluster
.with_connection_pool_size(100)
.with_keepalive(true)
```

---

## Database Tuning (if using)

### Query Optimization
```bash
# Analyze query plans
omnisystem db explain "SELECT ..."

# Add indexes
omnisystem db index create --column user_id

# Analyze statistics
omnisystem db analyze
```

### Cache Tuning
```bash
# Cache size
export CACHE_SIZE=1g

# Cache policy
export CACHE_POLICY=lru  # or lfu
```

---

## Monitoring Tuning

### Metrics Collection
```bash
# Reduce overhead
export METRICS_INTERVAL=30s  # Default 10s

# Selective metrics
export METRICS_INCLUDE=cpu,memory,requests
```

### Log Level
```bash
# Production: warn or error
export LOG_LEVEL=warn

# Development: debug
export LOG_LEVEL=debug
```

---

## Benchmarking Tuning

### Create Baseline
```bash
omnisystem bench > baseline.txt
```

### Compare Versions
```bash
omnisystem bench > new.txt
omnisystem compare-bench baseline.txt new.txt
```

### Profiling
```bash
omnisystem profile --cpu --output profile.txt
# Identify hot functions
```

---

## Configuration Summary

| Parameter | Default | Tuning | Effect |
|-----------|---------|--------|--------|
| Threads | Auto | +2x | Higher throughput |
| Heap Size | 1GB | +2x | Reduce GC |
| Batch Size | 32 | +4x | Better throughput |
| Cache Size | 256MB | +4x | Better hit rate |
| Heartbeat | 150ms | -50% | Faster failover |

---

## Workload-Based Tuning

### High Throughput
- Increase batch size
- Increase thread pool
- Increase cache size
- Reduce logging

### Low Latency
- Decrease thread pool
- Disable batching
- Increase CPU frequency
- Enable prefetching

### High Memory
- Reduce cache size
- Use f32 instead of f64
- Reduce batch size
- Enable compression

---

## Next Steps

- Performance: [PERFORMANCE.md](PERFORMANCE.md)
- Operations: [OPERATIONS.md](OPERATIONS.md)
- Troubleshooting: [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

---

**Tuning** - Optimize Omnisystem for your workload!
