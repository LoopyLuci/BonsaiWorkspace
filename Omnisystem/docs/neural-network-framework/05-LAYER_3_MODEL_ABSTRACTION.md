# Layer 3: Model Abstraction - Computational Graph Design

**Purpose**: Unified representation of neural network computations  
**Scope**: DAG-based computation graph, operation registry, type inference  
**Status**: 🚀 Ready for implementation

---

## COMPUTATIONAL GRAPH ARCHITECTURE

```
┌─────────────────────────────────────────────┐
│  Graph Representation (DAG)                 │
│  - Nodes: Operations                        │
│  - Edges: Tensor dependencies               │
│  - Metadata: Shape, dtype, device           │
│  - Versioning: Checkpoint/restore support   │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│  Type Inference & Shape Checking            │
│  - Automatic output shape computation       │
│  - Broadcasting rule enforcement            │
│  - Type promotion (fp32 + fp16 → fp32)      │
│  - Validation with clear error messages     │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│  Operation Registry (500+ operations)       │
│  - Core: MatMul, Conv, Add, etc.            │
│  - Activation: ReLU, GELU, Sigmoid, etc.    │
│  - Attention: SDPA, MultiHead, Flash, etc.  │
│  - Loss: CrossEntropy, MSE, FocalLoss, etc. │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│  Computation Execution                      │
│  - Forward pass (DAG evaluation)            │
│  - Backward pass (reverse-mode AD)          │
│  - Gradient computation                     │
│  - Device placement                         │
└─────────────────────────────────────────────┘
```

---

## COMPUTATIONAL GRAPH

### Node Structure

```rust
pub struct Node {
    // Identity
    pub id: String,
    pub name: String,
    
    // Operation
    pub operation: String,
    pub attributes: HashMap<String, Value>,
    
    // Connectivity
    pub inputs: Vec<String>,      // Input node IDs
    pub outputs: Vec<String>,     // Output node IDs
    
    // Type information
    pub input_types: Vec<TensorType>,
    pub output_types: Vec<TensorType>,
    
    // Execution
    pub device_placement: String,
    pub kernel_fn: KernelFunction,
    pub grad_fn: GradientFunction,
    
    // Metadata
    pub inplace: bool,
    pub requires_grad: bool,
    pub metadata: HashMap<String, Value>,
}
```

### Edge Structure

```rust
pub struct Edge {
    pub from_node: String,
    pub to_node: String,
    pub output_index: usize,  // Which output of from_node
    pub input_index: usize,   // Which input of to_node
    
    pub tensor_shape: Vec<usize>,
    pub tensor_dtype: String,
    pub tensor_device: String,
}
```

### Graph Structure

```rust
pub struct ComputationGraph {
    pub id: String,
    pub name: String,
    pub version: usize,
    
    pub nodes: HashMap<String, Node>,
    pub edges: Vec<Edge>,
    pub node_order: Vec<String>,  // Topological sort
    
    pub input_nodes: Vec<String>,
    pub output_nodes: Vec<String>,
    
    pub metadata: HashMap<String, Value>,
}

impl ComputationGraph {
    // Forward execution
    pub fn forward(&self, inputs: &[Tensor]) -> Vec<Tensor> { ... }
    
    // Backward pass
    pub fn backward(&self, grad_outputs: &[Tensor]) -> HashMap<String, Tensor> { ... }
    
    // Optimization
    pub fn optimize(&mut self, level: OptimizationLevel) { ... }
    
    // Versioning
    pub fn save_checkpoint(&self) -> String { ... }
    pub fn load_checkpoint(&mut self, id: &str) { ... }
    
    // Visualization
    pub fn to_dot(&self) -> String { ... }
    pub fn to_svg(&self) -> String { ... }
}
```

---

## TYPE INFERENCE SYSTEM

### Type Propagation

```
Forward pass:
  node1 = input(shape=[batch, 28, 28, 1], dtype=float32)
  node2 = conv2d(node1, filters=32, kernel_size=3)
    → output shape: [batch, 26, 26, 32]
    → output dtype: float32
  node3 = relu(node2)
    → shape preserved: [batch, 26, 26, 32]
    → dtype preserved: float32
```

### Type Inference Rules

```rust
pub struct TypeInference {
    rules: HashMap<String, InferenceRule>,
}

impl TypeInference {
    pub fn infer_output_type(
        &self,
        operation: &str,
        input_types: &[TensorType],
    ) -> Result<Vec<TensorType>, TypeError> {
        match operation {
            "matmul" => self.infer_matmul(input_types),
            "conv2d" => self.infer_conv2d(input_types),
            "add" | "sub" | "mul" | "div" => self.infer_elementwise(input_types),
            _ => Err(TypeError::UnknownOperation(operation.to_string())),
        }
    }
    
    fn infer_matmul(&self, input_types: &[TensorType]) -> Result<Vec<TensorType>> {
        // Input 0: [M, K], Input 1: [K, N]
        // Output: [M, N]
        if input_types[0].shape.len() != 2 || input_types[1].shape.len() != 2 {
            return Err(TypeError::InvalidMatmulShape);
        }
        
        let m = input_types[0].shape[0];
        let k = input_types[0].shape[1];
        let n = input_types[1].shape[1];
        
        if k != input_types[1].shape[0] {
            return Err(TypeError::ShapeMismatch);
        }
        
        Ok(vec![TensorType {
            shape: vec![m, n],
            dtype: self.promote_dtype(&input_types[0].dtype, &input_types[1].dtype),
            device: input_types[0].device.clone(),
            requires_grad: input_types[0].requires_grad || input_types[1].requires_grad,
        }])
    }
    
    fn infer_conv2d(&self, input_types: &[TensorType]) -> Result<Vec<TensorType>> {
        // Input: [B, H, W, C_in]
        // Weight: [K_h, K_w, C_in, C_out]
        // Output: [B, H_out, W_out, C_out]
        
        let batch = input_types[0].shape[0];
        let h = input_types[0].shape[1];
        let w = input_types[0].shape[2];
        let c_out = input_types[1].shape[3];
        
        // Assume stride=1, padding=0 for now
        let h_out = h - input_types[1].shape[0] + 1;
        let w_out = w - input_types[1].shape[1] + 1;
        
        Ok(vec![TensorType {
            shape: vec![batch, h_out, w_out, c_out],
            dtype: input_types[0].dtype.clone(),
            device: input_types[0].device.clone(),
            requires_grad: true,
        }])
    }
}
```

---

## OPERATION REGISTRY

### Core Operations (100+)

```
ELEMENT-WISE:
  add, subtract, multiply, divide, power, mod
  abs, neg, reciprocal, sqrt, square, exp, log, sin, cos, tan

MATRIX:
  matmul, batch_matmul, dot, outer, inner
  transpose, permute, reshape, expand, squeeze, flatten

CONVOLUTION:
  conv1d, conv2d, conv3d, depthwise_conv2d
  group_conv2d, transposed_conv2d, deformable_conv2d

POOLING:
  max_pool, avg_pool, adaptive_max_pool, adaptive_avg_pool
  lp_pool, fractional_max_pool

ACTIVATION:
  relu, gelu, silu, elu, selu, leaky_relu, threshold_relu
  sigmoid, tanh, softmax, log_softmax, softplus, mish, swish

NORMALIZATION:
  batch_norm, layer_norm, group_norm, instance_norm
  local_response_norm, spectral_norm

ATTENTION:
  scaled_dot_product_attention, multi_head_attention
  cross_attention, self_attention, flash_attention
  sparse_attention, linear_attention, performer_attention

LOSS:
  cross_entropy, mse, mae, huber, smooth_l1
  focal_loss, triplet_loss, contrastive_loss, nce_loss

REGULARIZATION:
  dropout, spatial_dropout, monte_carlo_dropout
  label_smoothing, mixup, cutout, cutmix
```

### Registering Custom Operations

```rust
pub fn register_custom_operation(
    registry: &mut OperationRegistry,
    name: &str,
    forward_fn: fn(&[Tensor]) -> Result<Vec<Tensor>>,
    backward_fn: fn(&[Tensor], &[Tensor]) -> Result<HashMap<usize, Tensor>>,
) {
    registry.register(Operation {
        name: name.to_string(),
        forward: forward_fn,
        backward: backward_fn,
        input_count: 2,
        output_count: 1,
    });
}
```

---

## AUTO-DIFFERENTIATION

### Gradient Computation

```rust
pub struct Gradient {
    pub node_id: String,
    pub gradient: Tensor,
}

pub fn backpropagate(
    graph: &ComputationGraph,
    output_grads: &[Tensor],
) -> HashMap<String, Tensor> {
    let mut gradients = HashMap::new();
    let mut grad_queue = Vec::new();
    
    // Initialize with output gradients
    for (i, node_id) in graph.output_nodes.iter().enumerate() {
        grad_queue.push((node_id.clone(), output_grads[i].clone()));
    }
    
    // Reverse topological order
    let reverse_order: Vec<_> = graph.node_order.iter().rev().cloned().collect();
    
    for node_id in reverse_order {
        if let Some(upstream_grad) = gradients.get(&node_id) {
            let node = &graph.nodes[&node_id];
            
            // Compute local gradients using grad_fn
            let input_tensors: Vec<_> = node.inputs.iter()
                .filter_map(|id| gradients.get(id))
                .cloned()
                .collect();
            
            match (node.grad_fn)(&[upstream_grad.clone()]) {
                Ok(local_grads) => {
                    for (i, input_id) in node.inputs.iter().enumerate() {
                        let grad = &local_grads[i];
                        gradients.entry(input_id.clone())
                            .and_modify(|g| *g = g.add(grad))
                            .or_insert(grad.clone());
                    }
                }
                Err(e) => eprintln!("Gradient computation error: {}", e),
            }
        }
    }
    
    gradients
}
```

---

## GRAPH OPTIMIZATION

### Analysis Passes

1. **Data Flow Analysis**
   - Identify which tensors are used where
   - Mark dead code (unused tensors)
   - Find reusable intermediate results

2. **Shape Analysis**
   - Compute all tensor shapes
   - Detect shape mismatches early
   - Identify opportunities for optimization

3. **Type Analysis**
   - Determine types for all tensors
   - Identify type conversions
   - Check for type mismatches

---

## IMPLEMENTATION CHECKLIST

### Phase 1: Graph Structure (Week 1-2)
- [ ] Node and Edge data structures
- [ ] ComputationGraph implementation
- [ ] Topological sorting
- [ ] Forward pass execution

### Phase 2: Type System (Week 2-3)
- [ ] TensorType structure
- [ ] Type inference rules (20+ operations)
- [ ] Broadcasting rules
- [ ] Error handling with clear messages

### Phase 3: Operation Registry (Week 3-4)
- [ ] OperationRegistry structure
- [ ] Register 50+ core operations
- [ ] Custom operation support
- [ ] Operation lookup optimization

### Phase 4: Auto-Differentiation (Week 4-5)
- [ ] Backward pass implementation
- [ ] Gradient accumulation
- [ ] Custom gradient support
- [ ] Numerical gradient checking

---

## SUCCESS CRITERIA

✅ Supports arbitrary DAG topologies  
✅ Automatic shape inference for all operations  
✅ <100ms graph construction for typical models  
✅ <1ms topological sort  
✅ 100% correct gradient computation  
✅ Clear error messages for all edge cases  

---

**Document**: Layer 3 - Model Abstraction  
**Version**: 1.0  
**Last Updated**: 2026-06-15
