# HELIX Compiler Architecture v1.0
## Unified GPU Compilation System

---

## 1. COMPILATION PIPELINE

```
HELIX Kernel Code
    ↓
[Lexer] → Tokens
    ↓
[Parser] → Kernel AST
    ↓
[Type Checker] → Typed Kernel AST
    ↓
[Target Analyzer] → GPU Capability Requirements
    ↓
[Optimizer] → Optimized GPU AST
    ↓
[Multi-Backend Code Gen] → SPIR-V / LLVM IR / PTX / HIP
    ↓
[Backend Compiler] → Machine Code (.cubin, .hsaco, .metal, etc.)
    ↓
[GPU Binary Package] → Executable Kernel
```

---

## 2. LEXER & PARSER

### 2.1 Kernel-Specific Tokens

```
Keywords: kernel, vertex_shader, fragment_shader, compute_shader
Attributes: @block_id, @thread_id, @global_id, @shared_memory, @global_memory, @texture
Qualifiers: const, shared_memory, global_memory, texture_memory
Operators: .*
```

### 2.2 Kernel AST

```
Kernel
├── name: string
├── input_params: Parameter[]
├── shared_memory: SharedMemoryDecl[]
├── body: KernelBody
├── block_size: (u32, u32, u32)
└── target_features: TargetFeature[]

KernelBody
├── statements: Statement[]
├── barriers: Barrier[]
├── atomic_ops: AtomicOperation[]
└── control_flow: ControlFlow
```

---

## 3. TYPE CHECKER FOR GPU CODE

### 3.1 GPU Type Constraints

```
fn type_check_kernel(kernel: Kernel) -> Result<TypedKernel, Error> {
    // Check thread indexing attributes
    if kernel.has_attribute("@block_id") {
        validate_block_id_type(kernel.get_param("@block_id"))
    }
    
    if kernel.has_attribute("@thread_id") {
        validate_thread_id_type(kernel.get_param("@thread_id"))
    }
    
    // Check memory space declarations
    for mem_decl in kernel.shared_memory {
        if mem_decl.size > gpu_max_shared_memory() {
            error("Shared memory exceeds device limit")
        }
    }
    
    // Check data types are GPU-compatible
    for type in kernel.all_types() {
        if not is_gpu_compatible_type(type) {
            error("Type {} not GPU-compatible", type)
        }
    }
    
    // Check barrier placement
    for barrier in kernel.barriers {
        if not all_threads_can_reach_barrier(barrier) {
            error("Barrier unreachable by some threads")
        }
    }
    
    // Check atomic operations
    for atomic_op in kernel.atomic_ops {
        if not type_supports_atomic(atomic_op.type) {
            error("Type {} doesn't support atomic operations", atomic_op.type)
        }
    }
    
    return Ok(TypedKernel(kernel, type_env))
}
```

---

## 4. TARGET ANALYSIS

### 4.1 GPU Capability Detection

```
fn analyze_target_requirements(kernel: TypedKernel) -> GPURequirements {
    requirements = GPURequirements {}
    
    // Compute capability needed
    if kernel.uses_double_precision() {
        requirements.min_compute_capability = "3.0"
    }
    
    if kernel.uses_warp_shuffle() {
        requirements.min_compute_capability = "3.0"
    }
    
    // Memory requirements
    shared_memory_used = kernel.estimate_shared_memory()
    global_memory_used = kernel.estimate_global_memory()
    
    requirements.shared_memory = shared_memory_used
    requirements.global_memory = global_memory_used
    
    // Parallelism characteristics
    requirements.min_block_size = kernel.min_block_size_for_correctness()
    requirements.optimal_block_size = kernel.estimate_optimal_block_size()
    
    return requirements
}
```

### 4.2 Hardware Abstraction

```
fn select_backend(kernel: TypedKernel, target: string) -> Backend {
    match target {
        "cuda" => {
            if requires_sm_90() { return CUDABackend("sm_90") }
            if requires_sm_80() { return CUDABackend("sm_80") }
            return CUDABackend("sm_70")
        },
        "hip" => return HIPBackend("gfx90a"),
        "metal" => return MetalBackend("macos-13.0"),
        "vulkan" => return VulkanBackend("1.3"),
        "webgpu" => return WebGPUBackend()
    }
}
```

---

## 5. CODE GENERATION

### 5.1 SPIR-V Generation (Universal IR)

```
fn generate_spirv(kernel: TypedKernel) -> SPIRVModule {
    module = SPIRVModule::new()
    
    // Capabilities
    module.add_capability(Capability::Shader)
    module.add_capability(Capability::ComputeShader)
    
    // Extension
    module.add_extension("SPV_KHR_vulkan_memory_model")
    
    // Memory model
    module.set_memory_model(AddressingModel::Logical, MemoryModel::Vulkan)
    
    // Entry point
    entry = module.add_function(kernel.name)
    entry.set_execution_model(ExecutionModel::GLCompute)
    entry.set_workgroup_size(kernel.block_size)
    
    // Function body
    bb_entry = entry.append_block("entry")
    builder = SPIRVBuilder(bb_entry)
    
    // Generate variable declarations
    for var_decl in kernel.variable_declarations {
        ptr_type = module.get_pointer_type(var_decl.type, StorageClass::Function)
        var = builder.create_variable(ptr_type)
    }
    
    // Generate instructions
    for stmt in kernel.body.statements {
        generate_statement(stmt, builder, module)
    }
    
    return module
}

fn generate_statement(stmt: Statement, builder: SPIRVBuilder, module: SPIRVModule) -> void {
    if stmt is Assignment {
        target = generate_expression(stmt.target, builder, module)
        value = generate_expression(stmt.value, builder, module)
        builder.create_store(target, value)
    }
    
    if stmt is IfStatement {
        cond = generate_expression(stmt.condition, builder, module)
        then_bb = builder.current_fn().append_block("then")
        else_bb = builder.current_fn().append_block("else")
        merge_bb = builder.current_fn().append_block("merge")
        
        builder.create_cond_branch(cond, then_bb, else_bb)
        
        builder.set_insertion_point(then_bb)
        generate_block(stmt.then_body, builder, module)
        builder.create_branch(merge_bb)
        
        builder.set_insertion_point(else_bb)
        generate_block(stmt.else_body, builder, module)
        builder.create_branch(merge_bb)
        
        builder.set_insertion_point(merge_bb)
    }
    
    if stmt is Barrier {
        builder.create_control_barrier(
            scope: Scope::Workgroup,
            semantics: MemorySemantics::WorkgroupMemory
        )
    }
    
    if stmt is AtomicOp {
        // Generate atomic instruction
        ptr = generate_expression(stmt.target, builder, module)
        value = generate_expression(stmt.value, builder, module)
        builder.create_atomic_operation(stmt.op, ptr, value)
    }
}
```

### 5.2 CUDA PTX Generation

```
fn generate_cuda_ptx(kernel: TypedKernel) -> PTXAssembly {
    ptx = ".version 7.2\n"
    ptx += ".target sm_80\n"
    ptx += ".address_size 64\n\n"
    
    ptx += ".visible .entry {}(\n".format(kernel.name)
    
    // Parameters
    for param in kernel.parameters {
        if param.is_pointer() {
            ptx += "  .param .u64 {}\n".format(param.name)
        } else {
            ptx += "  .param .{} {}\n".format(param.type.ptx_name(), param.name)
        }
    }
    
    ptx += ")\n{\n"
    
    // Register allocation
    registers = allocate_registers(kernel)
    
    // Shared memory declaration
    ptx += ".shared .align 4 .b8 shared[{}];\n".format(kernel.shared_memory_size)
    
    // Entry prologue
    ptx += "ld.param.u64 %rd0, [%0];\n"  // Load first pointer argument
    
    // Generate kernel body
    ptx += generate_kernel_body_ptx(kernel, registers)
    
    ptx += "ret;\n}\n"
    
    return PTXAssembly(ptx)
}
```

### 5.3 Metal Shader Generation

```
fn generate_metal(kernel: TypedKernel) -> MetalShader {
    code = "#include <metal_stdlib>\nusing namespace metal;\n\n"
    
    // Constants buffer
    code += "struct ComputeParams {\n"
    for param in kernel.parameters {
        code += "  {} {};\n".format(param.type.metal_name(), param.name)
    }
    code += "};\n\n"
    
    // Kernel function
    code += "kernel void {}(\n".format(kernel.name)
    code += "  uint3 gid [[thread_position_in_grid]],\n"
    code += "  uint3 lid [[thread_position_in_threadgroup]],\n"
    code += "  uint3 bid [[threadgroup_position_in_grid]],\n"
    code += "  device ComputeParams* params [[buffer(0)]]\n"
    code += ") {\n"
    
    // Generate kernel body in Metal C++
    code += generate_kernel_body_metal(kernel)
    
    code += "}\n"
    
    return MetalShader(code)
}
```

---

## 6. OPTIMIZATION

### 6.1 Kernel Optimization

```
fn optimize_kernel(kernel: TypedKernel) -> OptimizedKernel {
    // Loop optimizations
    kernel = unroll_loops(kernel)
    kernel = fuse_loops(kernel)
    
    // Register optimizations  
    kernel = reduce_register_pressure(kernel)
    kernel = cache_frequent_values(kernel)
    
    // Instruction optimization
    kernel = eliminate_dead_instructions(kernel)
    kernel = combine_memory_ops(kernel)
    kernel = reorder_instructions_for_latency(kernel)
    
    // Memory optimization
    kernel = optimize_memory_access_patterns(kernel)
    kernel = coalesce_global_memory_accesses(kernel)
    kernel = maximize_shared_memory_usage(kernel)
    
    // Parallelism optimization
    kernel = increase_instruction_level_parallelism(kernel)
    kernel = reduce_warp_divergence(kernel)
    
    return kernel
}

fn reduce_warp_divergence(kernel: TypedKernel) -> TypedKernel {
    // Detect divergent branches
    divergent_branches = find_divergent_branches(kernel)
    
    for branch in divergent_branches {
        // Try to reorder code to reduce divergence
        // Move convergence point earlier
        // Remove unnecessary branches
    }
    
    return kernel
}
```

---

## 7. MEMORY ANALYSIS

### 7.1 Memory Access Pattern Analysis

```
fn analyze_memory_access(kernel: TypedKernel) -> MemoryReport {
    report = MemoryReport {}
    
    // Global memory accesses
    for access in kernel.global_memory_accesses {
        if is_coalesceable(access) {
            report.coalesced_accesses += 1
        } else {
            report.uncoalesced_accesses += 1
            report.efficiency = lower_than_optimal()
        }
    }
    
    // Shared memory accesses
    bank_conflicts = detect_bank_conflicts(kernel.shared_memory_accesses)
    if bank_conflicts > 0 {
        report.warning("Bank conflicts detected: {}", bank_conflicts)
    }
    
    // Register usage
    registers_used = count_registers(kernel)
    occupancy = estimate_occupancy(kernel.block_size, registers_used)
    report.occupancy = occupancy
    
    return report
}
```

---

## 8. EXAMPLE: COMPLETE COMPILATION

```
HELIX Kernel:
──────────────
kernel vector_add(
    a: &[f32],
    b: &[f32],
    result: &mut [f32],
    @global_id idx: uint
) -> void {
    if idx < a.len() {
        result[idx] = a[idx] + b[idx]
    }
}

Targeting CUDA (sm_80):
═══════════════════════

Step 1: Parse & Type Check ✓
Step 2: Analyze Target Requirements ✓
  - Compute Capability: 8.0
  - Memory: ~48KB shared

Step 3: Generate SPIR-V (Intermediate)
  OpFunction OpTypeVoid
  ...
  OpAtomicLoad
  OpAtomicStore
  ...

Step 4: NVIDIA Compiler (SPIR-V → PTX)
  ptx -o vector_add.ptx vector_add.spv

Step 5: PTX Assembly:
  .version 7.2
  .target sm_80
  
  .visible .entry vector_add(
    .param .u64 a,
    .param .u64 b,
    .param .u64 result,
    .param .u32 idx
  ) {
    mov.u32 %r1, %tid.x;
    mov.u64 %rd1, a;
    mov.f32 %f1, [%rd1 + %r1*4];
    mov.u64 %rd2, b;
    mov.f32 %f2, [%rd2 + %r1*4];
    add.f32 %f3, %f1, %f2;
    mov.u64 %rd3, result;
    st.global.f32 [%rd3 + %r1*4], %f3;
    ret;
  }

Step 6: Compile to CUBIN (GPU Binary)
  ptxas -o vector_add.cubin vector_add.ptx

Result: vector_add.cubin (GPU-executable binary)
```

---

This architecture enables seamless GPU programming across all platforms with unified syntax and automatic optimization.
