# SYLVA Compiler Architecture v1.0
## ML/AI Data Language Compilation

---

## 1. PIPELINE

```
SYLVA Code
    ↓
[Parser] → Computation Graph AST
    ↓
[Type Inference] → Shape/Type Inference
    ↓
[Differentiation] → Compute Gradient Graphs
    ↓
[Graph Optimizer] → Fuse Ops, Eliminate Dead Code
    ↓
[Backend Selection] → GPU/CPU Target
    ↓
[Code Generator] → Kernel Code + Host Code
    ↓
[JIT Compiler] → Native Executable
    ↓
[Runtime] → GPU-Optimized Execution
```

---

## 2. SHAPE INFERENCE

```
fn infer_shapes(graph: AST, input_shapes: Map<string, Shape>) -> ShapeMap {
    shapes = input_shapes.clone()
    
    for node in graph.topological_order() {
        match node.operation {
            MatMul(a, b) => {
                shape_a = shapes[a]
                shape_b = shapes[b]
                shapes[node.output] = [shape_a[0], shape_b[1]]
            },
            Add(a, b) => {
                shapes[node.output] = broadcast_shapes(
                    shapes[a], shapes[b]
                )
            },
            ReLU(x) => {
                shapes[node.output] = shapes[x]
            },
            Conv2D(input, filters, stride, padding) => {
                let in_shape = shapes[input]
                let f_shape = shapes[filters]
                
                let h_out = (in_shape[2] + 2*padding[0] - f_shape[2]) / stride[0] + 1
                let w_out = (in_shape[3] + 2*padding[1] - f_shape[3]) / stride[1] + 1
                
                shapes[node.output] = [in_shape[0], f_shape[0], h_out, w_out]
            }
        }
    }
    
    return shapes
}

fn broadcast_shapes(shape1: Shape, shape2: Shape) -> Shape {
    // Align shapes from right
    while shape1.len() < shape2.len() {
        shape1.prepend(1)
    }
    while shape2.len() < shape1.len() {
        shape2.prepend(1)
    }
    
    result = []
    for (d1, d2) in zip(shape1, shape2) {
        if d1 == 1 {
            result.push(d2)
        } else if d2 == 1 {
            result.push(d1)
        } else if d1 == d2 {
            result.push(d1)
        } else {
            error("Shapes incompatible for broadcasting")
        }
    }
    
    return result
}
```

---

## 3. AUTOMATIC DIFFERENTIATION (AD)

```
fn build_gradient_graph(
    loss_node: Node,
    variables: [Node]
) -> Map<Node, Node> {
    gradients = {}
    
    // Start with gradient of loss w.r.t. loss = 1
    reverse_accumulator = {loss_node: 1.0}
    
    // Traverse graph in reverse topological order
    for node in graph.reverse_topological_order() {
        if node not in reverse_accumulator {
            continue
        }
        
        grad_output = reverse_accumulator[node]
        
        for input_node in node.inputs {
            // Apply chain rule
            local_grad = compute_local_gradient(node, input_node)
            
            if input_node not in reverse_accumulator {
                reverse_accumulator[input_node] = local_grad * grad_output
            } else {
                reverse_accumulator[input_node] += local_grad * grad_output
            }
        }
    }
    
    // Extract gradients for variables
    for var in variables {
        gradients[var] = reverse_accumulator[var]
    }
    
    return gradients
}

fn compute_local_gradient(node: Node, input_node: Node) -> Node {
    match node.operation {
        MatMul(a, b) => {
            if input_node == a {
                return MatMul(grad_output, transpose(b))
            } else {
                return MatMul(transpose(a), grad_output)
            }
        },
        Add(a, b) => {
            return grad_output  // Gradient passes through unchanged
        },
        ReLU(x) => {
            return grad_output * (node.output > 0)
        },
        Mul(a, b) => {
            if input_node == a {
                return grad_output * b
            } else {
                return grad_output * a
            }
        }
    }
}
```

---

## 4. GRAPH OPTIMIZATION

```
fn optimize_graph(graph: ComputationGraph) -> OptimizedGraph {
    // Operator fusion
    graph = fuse_operator_chains(graph)
    
    // Dead code elimination
    graph = eliminate_unused_nodes(graph)
    
    // Constant folding
    graph = fold_constants(graph)
    
    // Common subexpression elimination
    graph = eliminate_common_subexpressions(graph)
    
    // Memory optimization
    graph = optimize_memory_layout(graph)
    
    return graph
}

fn fuse_operator_chains(graph: ComputationGraph) -> ComputationGraph {
    // Fuse: MatMul -> Add -> ReLU into single fused kernel
    for subgraph in find_fusible_chains(graph) {
        fused_node = create_fused_op(subgraph)
        replace_subgraph(graph, subgraph, fused_node)
    }
    
    return graph
}

fn optimize_memory_layout(graph: ComputationGraph) -> ComputationGraph {
    // Analyze memory access patterns
    for node in graph.nodes {
        if node.is_memory_intensive() {
            preferred_layout = infer_memory_layout(node)
            node.set_layout_preference(preferred_layout)
        }
    }
    
    return graph
}
```

---

## 5. BACKEND CODE GENERATION

```
fn generate_cuda_code(graph: OptimizedGraph) -> CudaKernel {
    code = ""
    
    // Generate kernels for each operation
    for node in graph.nodes {
        kernel = generate_kernel(node)
        code += kernel
    }
    
    // Generate kernel launcher
    code += generate_launcher(graph)
    
    return CudaKernel(code)
}

fn generate_kernel(node: Node) -> string {
    kernel = ""
    
    match node.operation {
        MatMul(m, n, k) => {
            kernel = """
            __global__ void matmul_kernel(
                float* A, float* B, float* C,
                int M, int N, int K
            ) {
                int row = blockIdx.y * blockDim.y + threadIdx.y;
                int col = blockIdx.x * blockDim.x + threadIdx.x;
                
                if (row < M && col < N) {
                    float sum = 0.0f;
                    for (int k = 0; k < K; k++) {
                        sum += A[row * K + k] * B[k * N + col];
                    }
                    C[row * N + col] = sum;
                }
            }
            """
        },
        Conv2D(...) => {
            // Generate fused conv2d kernel
        }
    }
    
    return kernel
}
```

---

## 6. JIT COMPILATION

```
fn jit_compile(
    fn_def: FunctionDef,
    arg_shapes: [Shape]
) -> CompiledFunction {
    // Build computation graph for this function and input shapes
    graph = build_graph_for_args(fn_def, arg_shapes)
    
    // Specialize for concrete shapes
    graph = specialize_graph(graph, arg_shapes)
    
    // Optimize
    graph = optimize_graph(graph)
    
    // Generate code
    code = generate_backend_code(graph)
    
    // Compile to native code
    compiled = compile_to_machine_code(code)
    
    // Cache compiled function
    jit_cache[(fn_def, arg_shapes)] = compiled
    
    return compiled
}
```

---

## 7. EXAMPLE: NEURAL NETWORK COMPILATION

```
SYLVA Function:
─────────────────
fn forward(x: Tensor[32, 10], w1: Tensor[10, 5], w2: Tensor[5, 1]) -> Tensor[32, 1] {
    let h = relu(matmul(x, w1))
    return sigmoid(matmul(h, w2))
}

Step 1: Build Computation Graph
  x[32,10] → MatMul → [32,5] → ReLU → [32,5] → MatMul → [32,1] → Sigmoid → output[32,1]
  w1[10,5] ─┘                          w2[5,1] ─┘

Step 2: Shape Inference ✓
  All shapes verified

Step 3: Graph Optimization
  Fuse: MatMul + ReLU into single kernel
  Result: 2 fused ops instead of 4 separate ops

Step 4: Generate CUDA Code
  Kernel 1: matmul_relu_fused
  Kernel 2: matmul_sigmoid_fused

Step 5: JIT Compile to ptx
  Compile CUDA → PTX → CUBIN (GPU binary)

Step 6: Create Callable
  CompiledFunction with GPU memory management

Result: GPU-optimized neural network forward pass
```

---

This architecture enables SYLVA to provide automatic differentiation, GPU acceleration, and neural network support with transparent compilation.
