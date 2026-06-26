# HELIX Language Specification v1.0
## The Omnisystem GPU/Graphics/Compute Language

---

## 1. OVERVIEW

**HELIX** is a unified GPU programming language replacing CUDA, HIP, Metal Shaders, and OpenGL/Vulkan shaders. It provides:
- Single syntax for all GPU architectures (NVIDIA, AMD, Intel, Apple)
- Seamless CPU↔GPU interop (no data marshaling)
- Automatic optimization for target hardware
- Type-safe graphics pipelines
- Compile-time verification of GPU constraints

### Core Principles
1. **Unified Compute Model** - Same code targets all GPUs
2. **Type Safety** - GPU operations type-checked at compile time
3. **Zero-Copy** - Direct GPU memory access from TITAN
4. **Hardware Abstraction** - Write once, runs on any GPU
5. **Performance** - Near-hardware-level efficiency

---

## 2. BASIC SYNTAX

### 2.1 Kernel Functions

```helix
// Kernel: runs in parallel on all GPU threads
kernel matrix_multiply<T: Numeric>(
    a: Matrix<T>,
    b: Matrix<T>,
    out: &mut Matrix<T>,
    @block_id block_idx: uint3,
    @thread_id thread_idx: uint3
) -> void {
    let x = block_idx.x * block_size + thread_idx.x
    let y = block_idx.y * block_size + thread_idx.y
    
    if x < a.cols && y < b.rows {
        let mut sum: T = T::zero()
        for i in 0..a.cols {
            sum = sum + a[y, i] * b[i, x]
        }
        out[y, x] = sum
    }
}
```

### 2.2 Shader Functions

```helix
// Vertex shader
vertex_shader transform_vertices(
    @vertex_index index: u32,
    @instance_index instance: u32
) -> (
    @position: vec4<f32>,
    @color: vec4<f32>
) {
    let vertex = vertices[index]
    let instance_data = instances[instance]
    
    let world_pos = instance_data.transform * vertex.position
    let clip_pos = camera.projection * world_pos
    
    return (
        position: clip_pos,
        color: vertex.color
    )
}

// Fragment shader
fragment_shader color_fragment(
    @position pos: vec4<f32>,
    @color color: vec4<f32>,
    @uv uv: vec2<f32>
) -> @color: vec4<f32> {
    let tex_color = texture_sample(diffuse, sampler, uv)
    return color * tex_color
}

// Compute shader
compute_shader filter_image(
    @global_id id: uint3
) -> void {
    let uv = vec2<f32>(id.xy) / vec2<f32>(image_size)
    let src = texture_load(input_image, id.xy)
    
    let mut blur = vec4<f32>(0.0)
    for dy in -2..=2 {
        for dx in -2..=2 {
            let sample = texture_load(input_image, id.xy + vec2<i32>(dx, dy))
            blur = blur + sample
        }
    }
    
    texture_store(output_image, id.xy, blur / 25.0)
}
```

### 2.3 GPU Memory Spaces

```helix
// Register (per-thread, fastest)
let local_value: f32 = 3.14

// Shared memory (per-thread-block, fast, synchronized)
shared_memory shared_data: [f32; 256]

fn use_shared() {
    shared_data[thread_id.x] = thread_id.x as f32
    
    // Synchronize all threads in block
    barrier()
    
    let value = shared_data[(thread_id.x + 1) % 256]
}

// Global GPU memory (all threads, slower, persistent)
global_memory device_buffer: &mut [u32]

// Texture memory (optimized for 2D/3D access)
texture_memory input_texture: Texture2D<f32>
texture_memory output_surface: RWTexture2D<f32>

// Constant memory (read-only, broadcast)
constant_memory params: ComputeParams
```

### 2.4 Vector & Matrix Types

```helix
// Vectors
let v2: vec2<f32> = {1.0, 2.0}
let v3: vec3<i32> = {1, 2, 3}
let v4: vec4<f64> = {1.0, 2.0, 3.0, 4.0}

// Vector operations
let a = vec3<f32>{1.0, 2.0, 3.0}
let b = vec3<f32>{4.0, 5.0, 6.0}
let c = a + b  // {5.0, 7.0, 9.0}
let dot_prod = dot(a, b)  // 32.0
let cross_prod = cross(a, b)
let normalized = normalize(a)
let length = length(a)

// Matrices
let m4: mat4x4<f32> = identity()
let m3: mat3x3<f32> = m4.to_mat3()

// Matrix operations
let v = vec4<f32>{1.0, 2.0, 3.0, 1.0}
let transformed = m4 * v
let product = matrix_a * matrix_b
let determinant = det(m3)
let inverse = inverse(m4)
```

### 2.5 Atomics & Synchronization

```helix
kernel atomic_operations(
    @block_id block_idx: uint3,
    @thread_id thread_idx: uint3
) -> void {
    // Atomic operations
    atomic_add(&global_counter, 1)
    atomic_sub(&global_sum, 2)
    atomic_xor(&flags, 0xFF)
    atomic_compare_exchange(&lock, 0, 1)
    
    // Barriers
    barrier()  // Block-level sync
    memory_barrier()  // Memory sync
}

// Lock-free queue
struct LockFreeQueue<T> {
    head: atomic<u32>,
    tail: atomic<u32>,
    data: [T; 4096]
}

fn enqueue(q: &mut LockFreeQueue<i32>, value: i32) {
    loop {
        let tail = q.tail.load()
        let next = (tail + 1) % 4096
        
        if next != q.head.load() {
            if q.tail.compare_exchange(tail, next).is_ok() {
                q.data[tail] = value
                return
            }
        }
    }
}
```

### 2.6 Texture & Image Operations

```helix
// Texture types
texture_memory tex_color: Texture2D<f32>
texture_memory tex_normal: Texture2D<vec3<f32>>
texture_memory tex_depth: Texture2D<f32>

// Sampling
fn sample_texture(uv: vec2<f32>) -> vec4<f32> {
    let color = texture_sample(tex_color, linear_sampler, uv)
    return color
}

// Image operations
fn image_processing(id: uint3) {
    let pixel = texture_load(input_image, id.xy)
    
    // Apply filter
    let filtered = apply_gaussian_blur(pixel, id.xy)
    
    // Store result
    texture_store(output_image, id.xy, filtered)
}

// Texture formats
texture_memory tex_rgba8: Texture2D<vec4<u8>>
texture_memory tex_rg32f: Texture2D<vec2<f32>>
texture_memory tex_r32ui: Texture2D<u32>
```

### 2.7 Graphics Pipeline

```helix
// Define graphics pipeline
pipeline ForwardRenderer {
    // Vertex input
    vertex_input {
        position: vec3<f32> @location(0),
        normal: vec3<f32> @location(1),
        uv: vec2<f32> @location(2)
    }
    
    // Vertex shader
    vertex_shader: transform_vertices
    
    // Rasterization
    rasterization {
        cull_mode: Back,
        front_face: CCW,
        polygon_mode: Fill,
        line_width: 1.0
    }
    
    // Fragment shader
    fragment_shader: shading_fragment
    
    // Color attachments
    color_attachment {
        format: RGBA8,
        blend_op: Add,
        blend_src: One,
        blend_dst: OneMinusSrcAlpha
    }
    
    // Depth attachment
    depth_attachment {
        format: Depth32F,
        compare_op: Less,
        write_enabled: true
    }
    
    // Bindings
    binding {
        slot: 0,
        visibility: Vertex | Fragment,
        resource: UniformBuffer<FrameData>
    }
    
    binding {
        slot: 1,
        visibility: Fragment,
        resource: Texture2D<f32>
    }
}
```

---

## 3. DATA PARALLEL PATTERNS

### 3.1 Map Operation

```helix
kernel map_elements<T, U: Numeric>(
    input: &[T],
    output: &mut [U],
    @global_id idx: uint
) -> void {
    if idx < input.len() {
        output[idx] = transform(input[idx])
    }
}
```

### 3.2 Reduce Operation

```helix
kernel reduce_sum(
    input: &[f32],
    output: &mut [f32],
    @block_id block_idx: uint3,
    @thread_id thread_idx: uint3
) -> void {
    shared_memory partial_sums: [f32; 256]
    
    let idx = block_idx.x * 256 + thread_idx.x
    let tid = thread_idx.x
    
    // Load data
    if idx < input.len() {
        partial_sums[tid] = input[idx]
    } else {
        partial_sums[tid] = 0.0
    }
    
    barrier()
    
    // Tree reduction
    let mut stride: u32 = 128
    while stride > 0 {
        if tid < stride {
            partial_sums[tid] = partial_sums[tid] + partial_sums[tid + stride]
        }
        barrier()
        stride = stride / 2
    }
    
    // Write result
    if tid == 0 {
        atomic_add(&output[0], partial_sums[0])
    }
}
```

### 3.3 Scan Operation

```helix
kernel inclusive_scan(
    input: &[f32],
    output: &mut [f32],
    @block_id block_idx: uint3,
    @thread_id thread_idx: uint3
) -> void {
    shared_memory scan_data: [f32; 512]
    
    let tid = thread_idx.x
    let idx = block_idx.x * 512 + tid
    
    // Load input
    if idx < input.len() {
        scan_data[tid] = input[idx]
    } else {
        scan_data[tid] = 0.0
    }
    
    barrier()
    
    // Up-sweep phase
    let mut offset: u32 = 1
    while offset < 512 {
        if tid >= offset {
            scan_data[tid] = scan_data[tid] + scan_data[tid - offset]
        }
        offset = offset * 2
        barrier()
    }
    
    // Store result
    if idx < input.len() {
        output[idx] = scan_data[tid]
    }
}
```

---

## 4. TYPE SYSTEM

### 4.1 GPU-Compatible Types

```helix
// Numeric types
i32, u32, f32, f64

// Vector types
vec2<f32>, vec3<f32>, vec4<f32>
vec2<i32>, vec3<i32>, vec4<i32>
vec2<u32>, vec3<u32>, vec4<u32>

// Matrix types
mat2x2<f32>, mat3x3<f32>, mat4x4<f32>
mat2x3<f32>, mat3x4<f32>, etc.

// Array types
[T; size]  // Fixed size array in global memory

// Custom structs
struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>
}

struct Material {
    diffuse: vec3<f32>,
    specular: vec3<f32>,
    shininess: f32
}
```

### 4.2 Texture & Sampler Types

```helix
// Texture types
Texture1D<T>
Texture2D<T>
Texture3D<T>
TextureCube<T>

// Image types (read-write)
RWTexture1D<T>
RWTexture2D<T>
RWTexture3D<T>

// Sampler types
SamplerState          // Linear sampling
SamplerComparisonState // Depth comparison

// Buffer types
Buffer<T>             // Read-only structured buffer
RWBuffer<T>           // Read-write structured buffer
ByteAddressBuffer     // Raw byte buffer
```

---

## 5. OPTIMIZATION ATTRIBUTES

```helix
// Inline kernel
@inline
kernel small_kernel(...) { }

// Unroll loops
kernel loop_unrolling(...) {
    @unroll(4)
    for i in 0..16 {
        process(i)
    }
}

// Reduce block size
@block_size(256)
kernel efficient_kernel(...) { }

// Shared memory optimization
@shared_memory_size(16384)
kernel memory_intensive(...) { }

// Precision control
@precision(fp16)
kernel fast_computation(...) { }
```

---

## 6. COMPILATION TARGETS

```helix
// NVIDIA CUDA
@target("cuda")
kernel cuda_kernel(...) { }

// AMD HIP
@target("hip")
kernel hip_kernel(...) { }

// Intel DPC++
@target("dpc++")
kernel intel_kernel(...) { }

// Apple Metal
@target("metal")
kernel metal_kernel(...) { }

// Vulkan Compute
@target("vulkan")
kernel vulkan_kernel(...) { }

// WebGPU (browser)
@target("webgpu")
kernel web_kernel(...) { }
```

---

## 7. EXAMPLE: MATRIX MULTIPLICATION

```helix
kernel matrix_multiply(
    a: Matrix<f32>,
    b: Matrix<f32>,
    out: &mut Matrix<f32>,
    @block_id bid: uint3,
    @thread_id tid: uint3
) -> void {
    const TILE_SIZE: u32 = 16
    
    shared_memory tile_a: [f32; TILE_SIZE * TILE_SIZE]
    shared_memory tile_b: [f32; TILE_SIZE * TILE_SIZE]
    
    let row = bid.y * TILE_SIZE + tid.y
    let col = bid.x * TILE_SIZE + tid.x
    
    let mut result: f32 = 0.0
    
    for tile in 0..(a.cols / TILE_SIZE) {
        // Load tiles into shared memory
        let a_idx = row * a.cols + tile * TILE_SIZE + tid.x
        let b_idx = (tile * TILE_SIZE + tid.y) * b.cols + col
        
        if row < a.rows && (tile * TILE_SIZE + tid.x) < a.cols {
            tile_a[tid.y * TILE_SIZE + tid.x] = a.data[a_idx]
        }
        
        if (tile * TILE_SIZE + tid.y) < b.rows && col < b.cols {
            tile_b[tid.y * TILE_SIZE + tid.x] = b.data[b_idx]
        }
        
        barrier()
        
        // Multiply tiles
        for i in 0..TILE_SIZE {
            result = result + tile_a[tid.y * TILE_SIZE + i] * 
                             tile_b[i * TILE_SIZE + tid.x]
        }
        
        barrier()
    }
    
    // Write result
    if row < a.rows && col < b.cols {
        out.data[row * b.cols + col] = result
    }
}
```

---

## 8. INTEROP WITH TITAN

```helix
// HELIX kernel called from TITAN
kernel gpu_transform(
    data: &mut [f32],
    scale: f32
) -> void {
    let idx = global_thread_id()
    if idx < data.len() {
        data[idx] = data[idx] * scale
    }
}

// TITAN code
fn titan_main() -> void {
    let mut gpu_data: [f32] = [1.0, 2.0, 3.0, 4.0, 5.0]
    
    // Launch kernel
    launch_kernel(gpu_transform, {256, 1, 1}, {1, 1, 1}, gpu_data, 2.0)
    
    // Results available in gpu_data
    for value in gpu_data {
        println("{}", value)
    }
}
```

---

This specification enables HELIX to provide unified GPU programming across all architectures while maintaining maximum performance.
