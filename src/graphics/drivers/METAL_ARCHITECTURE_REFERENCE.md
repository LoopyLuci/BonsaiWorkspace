# Apple Metal Graphics Architecture Reference

## Table of Contents
1. [Unified Memory Architecture](#unified-memory-architecture)
2. [GPU Rendering Pipeline](#gpu-rendering-pipeline)
3. [Tile-Based Deferred Rendering (TBDR)](#tile-based-deferred-rendering)
4. [Compute Kernels](#compute-kernels)
5. [Command Buffer Model](#command-buffer-model)
6. [Frequency and Power Management](#frequency-and-power-management)
7. [Neural Engine Integration](#neural-engine-integration)
8. [Performance Characteristics](#performance-characteristics)
9. [Metal Shading Language (MSL)](#metal-shading-language)

---

## Unified Memory Architecture

### Memory Model

Apple Silicon uses a **unified memory architecture** where the GPU and CPU share the same physical memory space with automatic cache coherency.

```
┌─────────────────────────────────────────────────────┐
│              Unified Memory Space                    │
│  (100-400 GB/s bandwidth depending on chip)         │
│                                                     │
│  ┌─────────────────────────────────────────┐       │
│  │  CPU-Accessible Memory (DRAM)           │       │
│  │  ├─ CPU L3 Cache (8-24 MB)              │       │
│  │  ├─ CPU L2 Cache (256 KB per core)      │       │
│  │  └─ CPU L1 Cache (64 KB per core)       │       │
│  └─────────────────────────────────────────┘       │
│                    ↕ Cache Coherent                  │
│  ┌─────────────────────────────────────────┐       │
│  │  GPU-Accessible Memory (Same DRAM)      │       │
│  │  ├─ GPU L2 Cache (varies by GPU)        │       │
│  │  ├─ GPU Tile Memory (64 KB per core)    │       │
│  │  └─ GPU Register Files (32-bit wide)    │       │
│  └─────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────┘
```

### Memory Coherency

**Automatic Cache Coherency:**
- GPU reads/writes automatically synchronized with CPU memory
- No explicit cache flushes needed for basic operations
- Hardware handles coherency for transparent access
- Atomic operations supported for GPU-CPU synchronization

**Memory Barriers:**
```helix
// Ensure GPU-CPU coherency for complex scenarios
unified_memory.memory_barrier()?;

// Prefetch data from main memory to GPU
unified_memory.prefetch_to_gpu(address, size)?;
```

### Bandwidth Characteristics

| Chip | Bandwidth | Memory Type | Notes |
|------|-----------|-------------|-------|
| M1 | 100 GB/s | LPDDR4X | Standard configuration |
| M1 Pro | 200 GB/s | LPDDR5 | Dual-channel |
| M1 Max | 400 GB/s | LPDDR5 | Quad-channel |
| M2 | 100 GB/s | LPDDR5 | Single-channel |
| M2 Pro | 200 GB/s | LPDDR5 | Dual-channel |
| M2 Max | 400 GB/s | LPDDR5 | Quad-channel |
| M3 | 120 GB/s | LPDDR5X | Enhanced |
| M3 Pro | 240 GB/s | LPDDR5X | Dual-channel |
| M3 Max | 400 GB/s | LPDDR5X | Quad-channel |
| M4 | 120 GB/s | LPDDR5X | Standard |
| M4 Pro | 240 GB/s | LPDDR5X | Dual-channel |
| M4 Max | 400 GB/s | LPDDR5X | Quad-channel |

---

## GPU Rendering Pipeline

### Hardware Rendering Pipeline

```
┌─────────────────┐
│ Command Buffer  │
│  Submission     │
└────────┬────────┘
         │
    ┌────▼─────────────────────────┐
    │ Vertex Processing Stage      │
    │ - Load vertex data           │
    │ - Run vertex shader          │
    │ - Per-vertex calculations    │
    └────┬────────────────────────┘
         │
    ┌────▼─────────────────────────┐
    │ Tessellation (if enabled)    │
    │ - Patch processing           │
    │ - Barycentric coordinates    │
    └────┬────────────────────────┘
         │
    ┌────▼─────────────────────────┐
    │ Rasterization                │
    │ - Convert vertices to pixels │
    │ - Scan conversion            │
    └────┬────────────────────────┘
         │
    ┌────▼─────────────────────────┐
    │ Tile-Based Deferred Render   │
    │ - Bin primitives by tile     │
    │ - Create per-tile task list  │
    └────┬────────────────────────┘
         │
    ┌────▼─────────────────────────┐
    │ Fragment Processing (per-tile)
    │ - Load tile data to fast mem │
    │ - Run fragment shader        │
    │ - Per-fragment operations    │
    └────┬────────────────────────┘
         │
    ┌────▼─────────────────────────┐
    │ Render Target Operations     │
    │ - Blend/composite results    │
    │ - Write to main memory       │
    └────────────────────────────┘
```

### Pixel Format Support

```helix
pub enum PixelFormat {
    RGBA8Unorm,          // 8-bit UNORM per channel (32 bpp)
    RGBA16Float,         // 16-bit float per channel (64 bpp)
    RGBA32Float,         // 32-bit float per channel (128 bpp)
    RGB10A2Unorm,        // 10-bit RGB + 2-bit alpha (32 bpp)
    BGRAUnorm,           // Reverse channel order (32 bpp)
}
```

---

## Tile-Based Deferred Rendering (TBDR)

### Tile Architecture

Apple GPUs use tile-based deferred rendering for efficiency:

```
┌──────────────────────────────────────────────────┐
│           Full Screen (1920x1080)                │
│                                                  │
│  ┌────────┬────────┬─ ─ ─ ┬────────┐           │
│  │ Tile   │ Tile   │ ... │ Tile   │           │
│  │ 0      │ 1      │     │ N      │           │
│  │ 64x64  │ 64x64  │     │ 64x64  │           │
│  ├────────┼────────┼─ ─ ─ ┼────────┤           │
│  │ Tile   │ Tile   │ ... │ Tile   │           │
│  │ M      │ M+1    │     │ M+N    │           │
│  │ 64x64  │ 64x64  │     │ 64x64  │           │
│  └────────┴────────┴─ ─ ─ ┴────────┘           │
│                                                  │
│  30 × 17 = 510 tiles                           │
│  Each tile: 64x64 = 4,096 pixels               │
└──────────────────────────────────────────────────┘

Tile Memory Layout (per-tile, ~64KB):
┌──────────────────────────┐
│ Tile Memory (on-GPU)     │
├──────────────────────────┤
│ Color Attachment (32 KB) │  ← RGBA8 or better
│ Depth Attachment (16 KB) │  ← Depth/Stencil
│ Reserved/Scratch (16 KB) │  ← Working memory
└──────────────────────────┘
```

### TBDR Rendering Flow

1. **Binning Pass** (CPU-side or early GPU pass)
   - Determine which primitives affect each tile
   - Create per-tile command lists
   - Optimize primitive order

2. **Rendering Pass** (Per-tile GPU work)
   - Load tile data to fast on-GPU memory
   - Execute fragment shader for pixels in tile
   - Write results to tile memory

3. **Store Pass**
   - Write tile results back to main memory
   - Apply optional MSAA resolve
   - Transfer results to next stage

### Load/Store Operations

```helix
pub enum LoadAction {
    Load,       // Keep existing contents (most memory)
    Clear,      // Clear tile to color/depth (medium memory)
    DontCare,   // Ignore contents (least memory) ⭐ PREFERRED
}

pub enum StoreAction {
    Store,                  // Write to main memory
    MultisampleResolve,     // Resolve MSAA
    DontCare,              // Don't write (for intermediate) ⭐ PREFERRED
}
```

**Optimization Tips:**
- Use `LoadAction::DontCare` when not reading previous data
- Use `StoreAction::DontCare` when results aren't used later
- Minimize render target switching
- Batch similar renders together

---

## Compute Kernels

### Thread Hierarchy

```
┌─────────────────────────────────────────────────┐
│  Global Grid                                     │
│  (32 × 32 × 1 = 1024 threadgroups)              │
│                                                 │
│  ┌─────────────────────────────────────┐       │
│  │  Threadgroup (8 × 8 × 1 = 64)       │       │
│  │  ┌────────┐  ┌────────┐            │       │
│  │  │Thread  │  │Thread  │ ...        │       │
│  │  │(0,0,0) │  │(1,0,0) │            │       │
│  │  └────────┘  └────────┘            │       │
│  │  ┌────────┐  ┌────────┐            │       │
│  │  │Thread  │  │Thread  │ ...        │       │
│  │  │(0,1,0) │  │(1,1,0) │            │       │
│  │  └────────┘  └────────┘            │       │
│  │  ...                                │       │
│  └─────────────────────────────────────┘       │
└─────────────────────────────────────────────────┘
```

### Threadgroup Size Guidelines

| Task Type | Recommended Size | Total Threads | Notes |
|-----------|-----------------|---------------|-------|
| Image Processing | 16×16 | 256 | Good cache locality |
| Matrix Operations | 32×8 | 256 | Memory coalescing |
| Reduction | 256×1 | 256 | Warp synchronous |
| General Compute | 8×8×4 | 256 | 3D workloads |
| Maximum | 32×32×1 | 1024 | Resource limit |

### Shared Memory

- **Per-threadgroup limit:** 32 KB (typical)
- **Data types:** All SIMD-compatible types
- **Synchronization:** Threadgroup barriers
- **Performance:** Fast (~10 GB/s per core)

### Example Kernel

```metal
kernel void reduce(
    device const float *input [[buffer(0)]],
    device float *output [[buffer(1)]],
    uint2 gid [[thread_position_in_grid]],
    uint2 lid [[thread_position_in_threadgroup]],
    uint2 blocks [[threadgroups_per_grid]])
{
    // Shared memory for reduction
    threadgroup float shared[256];
    
    // Load data to shared memory
    shared[lid.x] = input[gid.x];
    threadgroup_barrier(mem_flags::mem_threadgroup);
    
    // Tree reduction
    for (uint stride = 128; stride > 0; stride >>= 1) {
        if (lid.x < stride) {
            shared[lid.x] += shared[lid.x + stride];
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);
    }
    
    // Write result
    if (lid.x == 0) {
        output[gid.y] = shared[0];
    }
}
```

---

## Command Buffer Model

### Command Submission Pipeline

```
Application Thread          GPU Queue            GPU Hardware
─────────────────────────────────────────────────────────────
    │
    ├─ Create CB
    │  └─→ 📋 Command Buffer
    │      (Encoding)
    │
    ├─ Add Cmds
    │  └─→ 📋 [RenderCmd, RenderCmd, ComputeCmd]
    │
    ├─ Commit
    │      │
    │      └─→ 📤 GPU Queue ──→ [Scheduled]
    │                            
    │                   GPU Processing
    │                   └─→ 🎮 Execute Cmds
    │                       [Completed]
    │
    └─ Wait
       └─→ ⏳ Block until [Completed]
```

### Command Buffer States

```helix
pub enum CommandStatus {
    Encoding,    // Building commands (writable)
    Committed,   // Submitted to queue (locked)
    Scheduled,   // GPU processing (executing)
    Completed,   // Results ready (readable)
    Error,       // Submission failed
}
```

### Typical Submission Pattern

```titan
// Create command buffer
let mut cmd_buf = driver.create_command_buffer();

// Add multiple commands
for geometry in scene.geometries {
    let cmd = MetalCommand {
        command_type: CommandType::RenderCommand,
        pipeline_id: Some(geometry.pipeline),
        vertex_buffer: Some(geometry.vertices),
        instance_count: geometry.instances,
    };
    cmd_buf.add_render_command(cmd);
}

// Commit for GPU execution
cmd_buf.commit()?;

// Optional: wait for completion
cmd_buf.wait_until_completed()?;
```

---

## Frequency and Power Management

### Power Modes

```
┌──────────────────────────────────────────┐
│     Power Mode Performance vs Power      │
│                                          │
│ High Performance                         │
│ ████████████████ 100% Freq  25W Power    │
│                                          │
│ Balanced                                 │
│ ███████████     75% Freq   15W Power     │
│                                          │
│ Power Saver                              │
│ ████████       50% Freq    8W Power      │
│                                          │
│ Thermal Reduction                        │
│ ████          25% Freq    3W Power       │
└──────────────────────────────────────────┘
```

### Thermal Throttling

```
Temperature (°C) │ GPU Frequency │ Status
─────────────────┼───────────────┼─────────────
40-50            │ 100%          │ Optimal
50-75            │ 100%          │ Normal
75+              │ 75%           │ Thermal warning
85+              │ 50%           │ THROTTLED
95+              │ 25%           │ Emergency
```

### API Usage

```helix
// Create frequency manager
let mut freq_mgr = GpuFrequencyManager::new(3000);  // M3 Max = 3000 MHz

// Set power mode
freq_mgr.set_power_mode(PowerMode::Balanced);

// Monitor temperature
freq_mgr.update_thermal_state(current_temp_c);
if freq_mgr.thermal_throttle_active {
    // Reduce workload
}

// Estimate power consumption
let power_mw = freq_mgr.estimate_power_mw();
```

---

## Neural Engine Integration

### Matrix Engine Architecture

```
┌──────────────────────────────────┐
│    Neural Engine (16 cores)      │
├──────────────────────────────────┤
│ ┌─────────────┐ ┌─────────────┐ │
│ │ Matrix Eng  │ │ Matrix Eng  │ │
│ │ 4×4 FP32    │ │ 8×8 Int8    │ │
│ └─────────────┘ └─────────────┘ │
│        ↕              ↕          │
│    Shared Memory / Cache         │
│        ↕              ↕          │
│ L2 Cache (Shared with GPU)       │
└──────────────────────────────────┘
```

### Compute Performance

| Data Type | Peak Throughput | Bandwidth Used |
|-----------|-----------------|----------------|
| FP32 | 11 TFLOPS (per core) | High |
| FP16 | 22 TFLOPS (per core) | Medium |
| INT8 | 44 TOPS (per core) | Low |
| INT4 | 88 TOPS (per core) | Very Low |

**Total (16 cores):**
- FP32: 176 TFLOPS peak
- FP16: 352 TFLOPS peak
- INT8: 704 TOPS peak

---

## Performance Characteristics

### Memory Latency

| Memory Level | Latency | Bandwidth |
|-------------|---------|-----------|
| Registers | 0 cycles | Per-thread |
| Tile Memory | 1-5 cycles | Per-core |
| L2 Cache | 10-20 cycles | ~400 GB/s |
| Main Memory | 100+ cycles | 100-400 GB/s |

### GPU Core Performance

| Chip | GPU Cores | Peak Perf (FP32) | Memory BW |
|------|-----------|------------------|-----------|
| M1 | 4-8 | 1.6-3.2 TFLOPS | 100 GB/s |
| M1 Pro | 10 | 4.0 TFLOPS | 200 GB/s |
| M1 Max | 16 | 6.4 TFLOPS | 400 GB/s |
| M2 | 10 | 3.6 TFLOPS | 100 GB/s |
| M3 Pro | 18 | 6.5 TFLOPS | 240 GB/s |
| M3 Max | 30 | 10.8 TFLOPS | 400 GB/s |
| M4 Max | 40 | 14.4 TFLOPS | 400 GB/s |

### Draw Call Overhead

- **CPU-GPU Sync:** ~100-200 μs
- **Command Buffer Submission:** ~10-50 μs
- **Per-Draw Overhead:** ~1-10 μs (very low)

**Optimization:** Batch draws together; prefer fewer large batches over many small draws.

---

## Metal Shading Language (MSL)

### MSL Language Features

**Attributes:**
```metal
// Vertex/Fragment shader inputs
vertex_id        // Vertex index (0, 1, 2, ...)
instance_id      // Instance index
stage_in         // Input data structure
position         // Output position (clip space)

// Fragment shader inputs
sample_id        // Sample index (MSAA)
frag_coord       // Fragment coordinates
```

**Address Spaces:**
```metal
__device           // Global GPU memory (device*)
__constant         // Read-only constants
__local            // Threadgroup shared memory
__threadgroup       // Threadgroup shared
__threadgroup_imageblock  // Per-tile fast memory
```

**Example: PBR Fragment Shader**

```metal
#include <metal_stdlib>
using namespace metal;

struct MaterialData {
    float4 albedo;
    float metallic;
    float roughness;
    float3 normal;
};

fragment float4 pbr_fs(
    float3 world_pos [[position]],
    float3 normal [[attribute(0)]],
    texture2d<float> albedo_tex [[texture(0)]],
    sampler tex_sampler [[sampler(0)]])
{
    // Sample material properties
    MaterialData mat;
    mat.albedo = albedo_tex.sample(tex_sampler, float2(0.5));
    mat.normal = normalize(normal);
    
    // Lighting calculation
    float3 light_dir = normalize(float3(1, 1, 1));
    float diffuse = max(dot(mat.normal, light_dir), 0.0);
    
    // PBR calculations would go here
    float3 radiance = mat.albedo.rgb * diffuse;
    
    return float4(radiance, mat.albedo.a);
}
```

---

## Summary Table

| Feature | M1 | M2 | M3 | M4 | A15 | A16 | A17P | A18 |
|---------|----|----|----|----|-----|-----|------|-----|
| GPU Cores | 4-16 | 8-19 | 8-30 | 10-40 | 5 | 5 | 6 | 6 |
| Memory BW | 100-400 | 100-400 | 120-400 | 120-400 | 100 | 100 | 120 | 120 |
| Ray Trace | ✗ | ✗ | ✓ (Pro/Max) | ✓ | ✗ | ✗ | ✗ | ✓ |
| Neural Cores | 16 | 16 | 16 | 16 | 16 | 16 | 16 | 16 |
| Mesh Shaders | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ |
| VRS | ✗ | ✗ | ✓ (Pro+) | ✓ | ✗ | ✗ | ✗ | ✓ |

---

## Further Reading

- [Metal Best Practices Guide](https://developer.apple.com/documentation/metal)
- [Metal Shading Language Specification](https://developer.apple.com/metal/)
- [Apple Silicon Developer Kit](https://developer.apple.com/)
