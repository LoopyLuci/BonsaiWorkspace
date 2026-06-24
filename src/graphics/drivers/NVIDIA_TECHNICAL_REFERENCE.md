# NVIDIA Graphics Driver - Technical Reference

## Module Structure

```
NvidiaGraphicsDriver.helix        (3,500 LOC)
├── GPU Architecture Definitions
│   ├── GpuArchitecture enum
│   ├── MemoryType enum
│   ├── TensorDataType enum
│   └── WarpState enum
├── GPU Device Information
│   ├── GpuDevice struct
│   ├── DriverInfo struct
│   └── Device detection/management
├── Streaming Multiprocessor (SM)
│   ├── StreamingMultiprocessor struct
│   ├── SM scheduling
│   └── SM resource allocation
├── Warp Execution Model
│   ├── Warp struct
│   ├── ThreadBlock struct
│   ├── Warp scheduling
│   └── Branch divergence handling
├── CUDA Kernel System
│   ├── CudaKernel struct
│   ├── KernelConfig struct
│   ├── Kernel loading/compilation
│   └── Thread scheduling
├── CUDA Memory Model
│   ├── CudaMemory struct
│   ├── MemoryHierarchy struct
│   ├── CacheConfig struct
│   └── Memory allocation/transfer
├── Tensor Core Operations
│   ├── TensorCore struct
│   ├── WmmaOperation struct
│   ├── MixedPrecisionConfig struct
│   └── Sparsity support
├── Ray Tracing Support
│   ├── RTCore struct
│   ├── BVH struct
│   ├── Ray/RayHit structs
│   └── BVH traversal
├── Occupancy & Scheduling
│   ├── OccupancyCalculator struct
│   ├── WarpScheduler struct
│   ├── BankConflictAnalyzer struct
│   └── Occupancy calculation
└── Performance Monitoring
    ├── GpuMetrics struct
    ├── PerformanceCounter struct
    └── Profiling support

NvidiaGpuRuntime.titan            (2,000 LOC)
├── Multi-GPU Coordination
│   ├── MultiGpuContext struct
│   ├── Peer-to-peer access
│   ├── Load balancing
│   └── Device synchronization
├── Kernel Optimization
│   ├── KernelOptimization struct
│   ├── OccupancyTuner struct
│   ├── Kernel fusion
│   └── Block configuration
├── Memory Optimization
│   ├── MemoryCoalescingOptimizer struct
│   ├── RegisterPressureAnalyzer struct
│   ├── BankConflictMitigator struct
│   └── Cache strategies
├── Tensor Core Optimization
│   ├── TensorCoreManager struct
│   ├── TensorContractionOptimizer struct
│   ├── MixedPrecisionTuner struct
│   └── Sparsity acceleration
├── Ray Tracing Optimization
│   ├── RtCoreLoadBalancer struct
│   ├── BvhTraversalOptimizer struct
│   └── RayBatchOptimizer struct
├── Performance Analysis
│   ├── GpuProfiler struct
│   ├── BottleneckAnalyzer struct
│   └── PerformanceReport struct
└── Automatic Optimization
    ├── AutoGpuOptimizer struct
    ├── HeuristicEngine struct
    └── Auto-tuning framework
```

## Data Structures

### GPU Device

```helix
pub struct GpuDevice {
    pub device_id: u32
    pub device_name: String
    pub architecture: GpuArchitecture
    pub compute_capability: (u32, u32)
    pub max_threads_per_block: u32
    pub max_blocks_per_sm: u32
    pub warp_size: u32                      // Always 32
    pub shared_memory_per_block: u32
    pub l1_cache_size_per_sm: u32
    pub l2_cache_size: u32
    pub register_file_size_per_sm: u32
    pub num_sms: u32
    pub max_warps_per_sm: u32
    pub memory_bandwidth: u64               // GB/s
    pub peak_fp32_performance: f64          // TFLOPS
    pub peak_fp16_performance: f64          // TFLOPS
    pub peak_tensor_performance: f64        // TFLOPS
    pub total_memory: u64
    pub free_memory: u64
    pub memory_type: String
    pub has_tensor_cores: bool
    pub has_rt_cores: bool
    pub ecc_enabled: bool
    pub power_limit_w: u32
    pub thermal_limit_c: u32
    pub gpu_clock_mhz: u32
    pub memory_clock_mhz: u32
}
```

**Fields Explanation:**
- `compute_capability`: Major.Minor version (9.0 for H100)
- `max_threads_per_block`: Up to 1024 threads
- `num_sms`: Streaming Multiprocessor count (132 for H100)
- `memory_bandwidth`: Theoretical maximum (3,352 GB/s for H100)
- `has_tensor_cores`: Specialized matrix units
- `has_rt_cores`: Ray tracing acceleration

### Thread Block

```helix
pub struct ThreadBlock {
    pub block_id: u32
    pub grid_id: u32
    pub block_dim: (u32, u32, u32)         // 3D thread dimensions
    pub block_size: u32                    // x*y*z total threads
    pub grid_dim: (u32, u32, u32)          // 3D grid dimensions
    pub shared_memory_used: u32
    pub register_per_thread: u32
    pub warps: Vec<Warp>
    pub block_state: String
}
```

**Block Characteristics:**
- Maximum 1024 threads per block
- Executes on one SM
- Shares L1 cache and shared memory
- Independent from other blocks

### Warp

```helix
pub struct Warp {
    pub warp_id: u32
    pub block_id: u32
    pub sm_id: u32
    pub thread_ids: Vec<u32>               // 32 threads
    pub state: WarpState
    pub pc: u32                            // Program counter
    pub active_mask: u32                   // Active lanes
    pub lane_mask: u32                     // Thread participation
    pub divergence_depth: u32              // Branch depth
}

pub enum WarpState {
    Active,                                // Executing
    Stalled,                               // Waiting
    Diverged,                              // Branch split
    Inactive,                              // Done
}
```

**Key Properties:**
- Fixed 32 threads per warp
- Lockstep execution
- Lane masks for branch handling
- Active mask tracks participating lanes

### CUDA Memory

```helix
pub struct CudaMemory {
    pub device_ptr: u64
    pub host_ptr: Option<u64>
    pub size_bytes: u64
    pub memory_type: MemoryType
    pub is_pitched: bool
    pub pitch_bytes: u64
    pub access_type: String
    pub is_pinned: bool
    pub allocation_time_us: u64
}

pub enum MemoryType {
    Global,                                // Main memory
    Shared,                                // Per-block cache
    Local,                                 // Register file
    Constant,                              // Cached read-only
    Texture,                               // Spatial cached
    Surface,                               // Write-capable
}
```

### Tensor Core Operation

```helix
pub struct WmmaOperation {
    pub m: u32                             // Rows in result
    pub n: u32                             // Cols in result
    pub k: u32                             // Reduction dimension
    pub input_type: TensorDataType
    pub output_type: TensorDataType
    pub a_ptr: u64                         // Matrix A
    pub b_ptr: u64                         // Matrix B
    pub c_ptr: u64                         // Accumulator
    pub d_ptr: u64                         // Result
    pub ld_a: u32                          // Leading dimension
    pub ld_b: u32
    pub ld_c: u32
    pub ld_d: u32
}

pub enum TensorDataType {
    FP32,                                  // 32-bit float
    FP16,                                  // 16-bit float
    TF32,                                  // TensorFloat-32
    BF16,                                  // Bfloat16
    INT8,                                  // 8-bit int
    INT4,                                  // 4-bit int
}
```

**WMMA Constraints:**
- M, N, K must be multiples of 8
- Operates on 16x16 tile by default
- Produces 16x16 output

### Ray Tracing Structures

```helix
pub struct BVH {
    pub bvh_id: String
    pub root_node_ptr: u64
    pub triangle_count: u32
    pub node_count: u32
    pub depth: u32
    pub surface_area: f64
    pub gpu_memory_used: u64
}

pub struct Ray {
    pub origin: (f32, f32, f32)
    pub direction: (f32, f32, f32)
    pub t_min: f32
    pub t_max: f32
}

pub struct RayHit {
    pub hit: bool
    pub t: f32                             // Distance
    pub triangle_id: u32
    pub barycoord: (f32, f32)              // UV coords
    pub normal: (f32, f32, f32)
}
```

## Memory Hierarchy

### Physical Organization

```
┌─────────────────────────────────────┐
│     Global Memory (GDDR6X/HBM2E)   │  10-100GB, ~100-900GB/s
├─────────────────────────────────────┤
│        L2 Cache (unified)           │  20-40MB, ~3TB/s
├─────────────────────────────────────┤
│ L1 Cache | Shared Memory | Tex Cache│  (per SM)
│ 128KB    │   96KB-192KB  │ 128KB    │
├─────────────────────────────────────┤
│    Register File (per SM)           │  256KB, fastest
└─────────────────────────────────────┘
```

### Access Characteristics

| Memory | Size | Bandwidth | Latency | Cached |
|---|---|---|---|---|
| Registers | 256KB/SM | N/A | 0 cycles | N/A |
| Shared | 96KB/block | ~9TB/s | 0-3 cycles | N/A |
| L1 Cache | 128KB/SM | ~9TB/s | ~4 cycles | Yes |
| L2 Cache | 20-40MB | ~3TB/s | ~20 cycles | Yes |
| Global | 10-100GB | ~900GB/s | ~400-600 cycles | Optional |
| Constant | 64KB | ~900GB/s (cached) | ~4 cycles | Yes |

## Memory Coalescing

### Optimal Pattern (Coalesced)
```
Thread 0:  reads from address 0
Thread 1:  reads from address 4
Thread 2:  reads from address 8
...
Thread 31: reads from address 124

Result: 1 transaction (32 threads × 4 bytes = 128 bytes)
```

### Suboptimal Pattern (Non-coalesced)
```
Thread 0:  reads from address 0
Thread 1:  reads from address 256
Thread 2:  reads from address 512
...
Thread 31: reads from address 7680

Result: 32 transactions (each thread separate)
Efficiency: ~1/32 = ~3%
```

### Coalescing Rules
1. Access stride must be contiguous per warp
2. Warp size = 32 threads
3. Cache line = 128 bytes
4. Max 4 transactions per warp for full line coalescing
5. Misaligned access still coalesces but uses more transactions

## Shared Memory Bank Conflicts

### Configuration
- 32 banks per SM
- 4 bytes per bank
- Bank width: 4 bytes
- Conflicts occur on same-cycle access to same bank

### Bank Assignment
```
Address 0-3:    Bank 0
Address 4-7:    Bank 1
Address 8-11:   Bank 2
...
Address 124-127: Bank 31
Address 128-131: Bank 0 (wraps)
```

### Conflict Types
1. **No Conflict**: All threads access different banks → 1 transaction
2. **2-way Conflict**: 2 threads → same bank → 2 transactions
3. **Full Serialization**: All threads → same bank → 32 transactions

## Occupancy Calculation

### Limiting Factors

```
Blocks per SM = min(
    max_blocks_per_sm,                                    // 32
    register_limit / (block_size × regs_per_thread),     // Register
    shared_memory_limit / shared_memory_per_block         // Shared mem
)

Warps per SM = blocks_per_sm × (block_size / 32)

Occupancy = (warps_per_sm / max_warps_per_sm) × 100%
```

### Example Calculation (Hopper)
```
Device limits:
- max_blocks_per_sm: 32
- max_warps_per_sm: 48
- register_file: 256KB per SM
- shared_memory: 96KB per block

Kernel config:
- block_size: 256 threads
- registers_per_thread: 32
- shared_memory: 8KB

Register constraint:
- register_used = 256 × 32 = 8192 registers
- register_limit = 256K / 32-bit = 65536 registers
- blocks_by_register = 65536 / 8192 = 8 blocks

Shared memory constraint:
- blocks_by_shared = 96K / 8K = 12 blocks

Occupancy limiting factor: 8 blocks
- warps = 8 × (256 / 32) = 64 warps
- occupancy = 64 / 48 = ~133% (capped at max)
```

## Warp Scheduling

### Scheduling Policies

1. **Round-Robin (Default)**
   - Fair distribution
   - Simple implementation
   - Good for mixed workloads

2. **Greedy-Then-Oldest**
   - Prioritize ready warps
   - Reduce stalls
   - Better latency hiding

3. **Two-Level Scheduler**
   - Primary: scheduler picks warp
   - Secondary: instruction level parallelism
   - Modern GPUs use this

### Stall Reasons
```
Memory Stall:        Waiting for L1/L2/Global memory
Dependency Stall:    Waiting for operand ready
Control Stall:       Branch prediction miss
Pipeline Stall:      Instruction pipeline dependency
Texture Stall:       Texture cache miss
Other Stall:         Miscellaneous
```

## Tensor Core Architecture

### WMMA Dimensions

For Tensor Cores, common configurations:
```
M=16, N=16, K=16      # Standard 16x16 operation
M=16, N=16, K=32      # Extended K dimension
M=32, N=8, K=16       # Different aspect ratio
M=8, N=32, K=16       # Transposed aspect
```

### Data Flow

```
┌─────────┐         ┌──────────┐         ┌──────────┐
│ Matrix A│         │ Matrix B │         │ Matrix C │
│(M×K)   │         │(K×N)    │         │(M×N)    │
└────┬────┘         └────┬─────┘         └────┬─────┘
     │ Load from    │ Load from    │ Load from
     │ Global/Shared│ Global/Shared│ Global/Shared
     │              │              │
     └──────────────┬───────────────┘
                    │
            ┌───────▼────────┐
            │  Tensor Core   │
            │  (WMMA Unit)   │
            └───────┬────────┘
                    │
            ┌───────▼────────┐
            │ Store Result D │
            │   (M×N)        │
            └────────────────┘
```

### Mixed Precision Example (FP16 input → FP32 output)

```
Input:  A[16×16] FP16 (256 bytes)
        B[16×16] FP16 (256 bytes)
        C[16×16] FP32 (1024 bytes)

Compute: D = α(A×B) + βC

Output: D[16×16] FP32 (1024 bytes)

Speedup: ~2× vs pure FP32
Memory: ~50% less for inputs
```

## API Reference Summary

### Device Management
| Function | Purpose |
|---|---|
| `detect_gpus()` | Query available GPUs |
| `set_device(id)` | Select active GPU |
| `get_device_properties(id)` | Query device specs |
| `cuda_device_synchronize()` | Wait for GPU |

### Memory Management
| Function | Purpose |
|---|---|
| `cuda_malloc(size)` | Allocate device memory |
| `cuda_malloc_host(size)` | Allocate pinned host memory |
| `cuda_malloc_pitch(w, h)` | Allocate 2D pitched memory |
| `cuda_memcpy_host_to_device(dst, src, size)` | H2D transfer |
| `cuda_memcpy_device_to_host(dst, src, size)` | D2H transfer |
| `cuda_free(ptr)` | Free memory |

### Kernel Execution
| Function | Purpose |
|---|---|
| `cuda_load_kernel(name, ptx)` | Load PTX kernel |
| `cuda_launch_kernel(id, grid, block, shared)` | Execute kernel |
| `schedule_threads(kernel)` | Internal scheduling |

### Warp Operations
| Function | Purpose |
|---|---|
| `create_warp(block, sm, warp)` | Create warp |
| `detect_warp_divergence(warp, conditions)` | Handle branches |
| `analyze_bank_conflicts(warp, addresses)` | Check conflicts |
| `analyze_memory_coalescing(warp, base, offsets)` | Check coalescing |

### Tensor Operations
| Function | Purpose |
|---|---|
| `wmma_execute(operation)` | Execute tensor operation |
| `set_mixed_precision_config(in, compute, out)` | Configure precision |
| `enable_sparsity_acceleration(ratio)` | Enable sparsity |

### Ray Tracing
| Function | Purpose |
|---|---|
| `create_bvh(id, vertices)` | Create BVH structure |
| `trace_ray(bvh, ray)` | Single ray trace |
| `trace_ray_batch(bvh, rays)` | Batch ray tracing |
| `optimize_bvh_traversal(bvh)` | Optimize BVH |

### Performance Monitoring
| Function | Purpose |
|---|---|
| `cuda_event_record(name)` | Create event |
| `cuda_event_synchronize(name)` | Wait for event |
| `cuda_event_elapsed_time(start, end)` | Get time |
| `get_metrics()` | Get performance data |
| `enable_profiling()` | Start profiling |
| `get_memory_stats()` | Query memory usage |
| `get_occupancy()` | Get occupancy info |

### Streams and Synchronization
| Function | Purpose |
|---|---|
| `cuda_stream_create()` | Create async stream |
| `cuda_stream_synchronize(id)` | Wait for stream |
| `cuda_stream_destroy(id)` | Destroy stream |

### Power Management
| Function | Purpose |
|---|---|
| `set_power_state(level)` | Set power mode |
| `get_gpu_temperature()` | Get current temp |
| `get_power_usage()` | Get power draw |
| `get_clock_speeds()` | Get clock frequencies |

### Runtime Optimization (TITAN)
| Function | Purpose |
|---|---|
| `initialize_gpu_runtime(device_count)` | Initialize runtime |
| `analyze_kernel(kernel_id)` | Optimize kernel |
| `analyze_memory_access()` | Memory analysis |
| `analyze_register_pressure(kernel_id)` | Register analysis |
| `analyze_bank_conflicts()` | Shared mem analysis |
| `optimize_tensor_contraction(shapeA, shapeB)` | Tensor optimization |
| `balance_load()` | Load balance multi-GPU |
| `apply_auto_optimizations()` | Auto-optimize |
| `generate_performance_report()` | Performance report |

## Implementation Notes

### Thread Safety
- Driver is single-threaded per context
- Use separate contexts for multi-threaded access
- Stream operations are thread-safe

### Memory Alignment
- Global memory: 16-byte alignment recommended
- Shared memory: 4-byte per thread recommended
- Coalescing: 128-byte cache line alignment

### Performance Tips

1. **Maximize Occupancy**: Aim for 75%+
2. **Minimize Divergence**: Keep control flow uniform in warps
3. **Coalesce Memory**: Sequential thread access patterns
4. **Hide Latency**: Use many warps, unroll loops
5. **Use Shared Memory**: For reducing global memory traffic
6. **Avoid Bank Conflicts**: Padding and careful layout

## Limitations

1. **Block Scheduling**: Non-preemptive, runs to completion
2. **Memory Model**: No cache coherency between blocks
3. **Warp Size**: Fixed 32 threads (cannot be changed)
4. **SM Limit**: Cannot guarantee specific block placement
5. **Shared Memory**: Limited per-block (96KB-192KB)
6. **Register File**: Limited per-thread (~255 registers)

## Debugging

Enable profiling for detailed metrics:
```helix
driver.enable_profiling()?;
// Run kernels
let metrics = driver.get_metrics();
println!("L1 Hit Rate: {:.1}%", metrics.l1_cache_hit_rate * 100.0);
println!("Divergence: {:.1}%", metrics.warp_divergence_rate * 100.0);
```

## Verification Checklist

- [ ] GPU detected correctly
- [ ] Memory allocations succeed
- [ ] Kernels launch without errors
- [ ] Memory transfers complete
- [ ] Performance metrics reasonable
- [ ] No thermal throttling
- [ ] Occupancy acceptable
- [ ] No warp divergence issues
- [ ] Memory coalescing good
- [ ] Bank conflicts minimal
