# Layer 2: High-Level API Design

**Purpose**: User-friendly neural network construction and training  
**Scope**: Declarative APIs, auto-diff, agent-driven design, deployment  
**Status**: 🚀 Ready for implementation

---

## API DESIGN PRINCIPLES

1. **Simplicity First**: Neural network in 5 lines of code
2. **Declarative**: Define what, not how
3. **Composable**: Mix and match layers
4. **Extensible**: Add custom layers easily
5. **Learnable**: Progressive complexity
6. **Type-Safe**: Catch errors at definition time
7. **Agent-Ready**: Automatable for AI systems

---

## SIMPLE EXAMPLE (TITAN)

```titan
// Define a simple classifier in 5 lines
let model = ModelBuilder::new("classifier")
    .add_input([null, 28, 28, 1])
    .add_conv2d(32, [3, 3], "relu")
    .add_dense(10, "softmax")
    .compile("adam", "categorical_crossentropy", ["accuracy"])
```

---

## DECLARATIVE JSON/YAML API

```yaml
model:
  name: transformer-classifier
  architecture:
    layers:
      - type: input
        shape: [null, 512]
      - type: embedding
        vocab_size: 50000
        embedding_dim: 512
      - type: transformer_block
        num_layers: 6
        embed_dim: 512
        num_heads: 8
        ff_dim: 2048
        dropout: 0.1
      - type: global_average_pooling_1d
      - type: dense
        units: 10
        activation: softmax
        
  training:
    optimizer:
      type: adam
      learning_rate: 0.001
      weight_decay: 0.0001
    loss: categorical_crossentropy
    metrics: [accuracy, f1_macro]
    batch_size: 32
    epochs: 100
    early_stopping:
      monitor: val_loss
      patience: 10
```

---

## PROGRAMMATIC API (TITAN)

```titan
pub struct ModelBuilder {
    name: String
    layers: Array[LayerConfig]
    training_config: TrainingConfig
    optimization_config: OptimizationConfig
}

impl ModelBuilder {
    pub fn new(name: String) -> Self { ... }
    
    // Input/output layers
    pub fn add_input(mut self: Self, shape: Array[Int]) -> Self { ... }
    pub fn add_output(mut self: Self, activation: String) -> Self { ... }
    
    // Core layers
    pub fn add_dense(mut self: Self, units: Int, activation: String) -> Self { ... }
    pub fn add_conv2d(mut self: Self, filters: Int, kernel_size: Array[Int]) -> Self { ... }
    pub fn add_conv1d(mut self: Self, filters: Int, kernel_size: Int) -> Self { ... }
    pub fn add_lstm(mut self: Self, units: Int, return_sequences: Bool) -> Self { ... }
    pub fn add_gru(mut self: Self, units: Int) -> Self { ... }
    pub fn add_embedding(mut self: Self, vocab_size: Int, embedding_dim: Int) -> Self { ... }
    
    // Attention layers
    pub fn add_attention(mut self: Self, num_heads: Int, key_dim: Int) -> Self { ... }
    pub fn add_transformer_block(mut self: Self, embed_dim: Int, num_heads: Int, 
                                 ff_dim: Int, dropout: Float) -> Self { ... }
    
    // Normalization layers
    pub fn add_batch_norm(mut self: Self) -> Self { ... }
    pub fn add_layer_norm(mut self: Self) -> Self { ... }
    pub fn add_group_norm(mut self: Self, num_groups: Int) -> Self { ... }
    
    // Regularization layers
    pub fn add_dropout(mut self: Self, rate: Float) -> Self { ... }
    pub fn add_spatial_dropout(mut self: Self, rate: Float) -> Self { ... }
    
    // Pooling layers
    pub fn add_max_pool(mut self: Self, pool_size: Array[Int]) -> Self { ... }
    pub fn add_avg_pool(mut self: Self, pool_size: Array[Int]) -> Self { ... }
    pub fn add_global_max_pool(mut self: Self) -> Self { ... }
    pub fn add_global_avg_pool(mut self: Self) -> Self { ... }
    
    // Utility layers
    pub fn add_flatten(mut self: Self) -> Self { ... }
    pub fn add_reshape(mut self: Self, shape: Array[Int]) -> Self { ... }
    pub fn add_residual_connection(mut self: Self) -> Self { ... }
    
    // Training configuration
    pub fn with_optimizer(mut self: Self, optimizer: String, lr: Float) -> Self { ... }
    pub fn with_loss(mut self: Self, loss: String) -> Self { ... }
    pub fn with_metrics(mut self: Self, metrics: Array[String]) -> Self { ... }
    pub fn with_regularization(mut self: Self, l1: Float, l2: Float) -> Self { ... }
    pub fn with_batch_size(mut self: Self, batch_size: Int) -> Self { ... }
    pub fn with_early_stopping(mut self: Self, patience: Int, monitor: String) -> Self { ... }
    
    // Optimization configuration
    pub fn with_precision(mut self: Self, precision: String) -> Self { ... }  // mixed_float16
    pub fn with_quantization(mut self: Self, method: String, target_dtype: String) -> Self { ... }
    pub fn with_pruning(mut self: Self, target_sparsity: Float) -> Self { ... }
    pub fn with_distillation(mut self: Self, teacher_model: CompiledModel) -> Self { ... }
    
    // Compilation
    pub fn compile(self: Self) -> CompiledModel { ... }
}

// Usage example
let model = ModelBuilder::new("resnet50")
    .add_input([null, 224, 224, 3])
    
    // First block
    .add_conv2d(64, [7, 7])
    .add_batch_norm()
    .add_activation("relu")
    .add_max_pool([3, 3])
    
    // Residual blocks (repeat x3 with different depths)
    .add_residual_block(64, 3)
    .add_residual_block(128, 4)
    .add_residual_block(256, 6)
    .add_residual_block(512, 3)
    
    // Classification head
    .add_global_avg_pool()
    .add_dense(1000, "softmax")
    
    .with_optimizer("adam", 0.001)
    .with_loss("categorical_crossentropy")
    .with_metrics(["accuracy", "top_5_accuracy"])
    .with_regularization(0.0, 0.0001)
    .with_batch_size(256)
    .with_precision("mixed_float16")
    .compile()
```

---

## AUTO-DIFFERENTIATION ENGINE

```
pub struct AutoDiff {
    graph: ComputationGraph
    tape: Array[Operation]
    
    pub fn forward(mut self: Self, inputs: Object) -> Object {
        // Execute forward pass, recording operations
        let result = self.graph.execute_forward(inputs)
        self.tape = self.graph.extract_operations()
        result
    }
    
    pub fn backward(self: Self, loss: Tensor) -> Object {
        // Reverse-mode differentiation (backpropagation)
        // Implements chain rule: ∂loss/∂w = (∂loss/∂output) * (∂output/∂w)
        let mut gradients = {}
        let mut grad_stack = [(self.graph.output(), loss)]
        
        while grad_stack.is_not_empty() {
            let (node, upstream_grad) = grad_stack.pop()
            let local_grads = node.compute_gradients(upstream_grad)
            
            for (input, grad) in node.inputs.zip(local_grads) {
                if gradients.contains(input.id()) {
                    gradients[input.id()] = gradients[input.id()].add(grad)
                } else {
                    gradients[input.id()] = grad
                }
                grad_stack.push((input, grad))
            }
        }
        
        gradients
    }
    
    pub fn jacobian(self: Self, inputs: Object, output_idx: Int) -> Tensor {
        // Matrix of all first-order partial derivatives
        let mut jacobian = array()
        for i in range(inputs.shape[0]) {
            let grad = self.backward(one_hot(output_idx))
            jacobian.push(grad)
        }
        jacobian
    }
    
    pub fn hessian(self: Self, inputs: Object) -> Tensor {
        // Matrix of second-order partial derivatives
        // Compute Jacobian of Jacobian
        ...
    }
}
```

---

## AGENT-DRIVEN MODEL BUILDING

```titan
pub struct ModelAgent {
    llm: LLMClient
    framework: FrameworkClient
    dataset_analyzer: DatasetAnalyzer
    performance_tracker: PerformanceTracker
    
    pub fn design_model(mut self: Self, dataset_description: String,
                        task_type: String, constraints: Object) -> CompiledModel {
        // Step 1: Analyze dataset
        let analysis = self.dataset_analyzer.analyze(dataset_description)
        
        // Step 2: LLM generates architecture
        let prompt = format!(
            "Design optimal neural network for {} task.
             Dataset: {} features, {} samples, {} classes
             Constraints: {}",
            task_type, analysis.num_features(), analysis.num_samples(),
            analysis.num_classes(), constraints
        )
        let architecture = self.llm.generate(prompt)
        
        // Step 3: Validate architecture
        let validation = self.framework.validate(architecture)
        if !validation.is_valid() {
            return self.design_model_iterative(validation.errors(), constraints)
        }
        
        // Step 4: Build and compile
        ModelBuilder::from_architecture(architecture).compile()
    }
    
    pub fn optimize_model(mut self: Self, model: CompiledModel,
                         objectives: Object) -> CompiledModel {
        // Get baseline
        let baseline = self.performance_tracker.benchmark(&model)
        
        // Generate optimization strategies via LLM
        let strategies = self.llm.generate_optimizations(baseline, objectives)
        
        // Evaluate and select best
        let mut best_model = model.clone()
        let mut best_score = baseline.score()
        
        for strategy in strategies {
            let optimized = self.apply(strategy, &best_model)
            let perf = self.performance_tracker.benchmark(&optimized)
            if perf.score() > best_score {
                best_model = optimized
                best_score = perf.score()
            }
        }
        
        best_model
    }
}
```

---

## TRAINING LOOP

```titan
pub struct Trainer {
    model: CompiledModel
    optimizer: Optimizer
    loss_fn: Function
    metrics: Array[Metric]
    
    pub fn train(mut self: Self, dataset: DataLoader,
                 epochs: Int, device: String) -> TrainingHistory {
        let mut history = TrainingHistory::new()
        
        for epoch in range(epochs) {
            let mut epoch_loss = 0.0
            let mut epoch_metrics = {}
            
            for batch in dataset {
                // Forward pass
                let predictions = self.model.forward(batch.data, device)
                let loss = self.loss_fn(predictions, batch.labels)
                
                // Backward pass
                let gradients = loss.backward()
                
                // Update weights
                self.optimizer.step(gradients)
                
                // Track metrics
                epoch_loss += loss.item()
                for metric in self.metrics {
                    epoch_metrics[metric.name()] = metric.update(predictions, batch.labels)
                }
            }
            
            history.record_epoch(epoch, epoch_loss, epoch_metrics)
            
            if epoch % 10 == 0 {
                println!("Epoch {}: loss={}, {}", epoch, epoch_loss, epoch_metrics)
            }
        }
        
        history
    }
}
```

---

## INFERENCE

```titan
pub struct Predictor {
    model: CompiledModel
    device: String
    batch_size: Int
    
    pub fn predict(self: Self, inputs: Tensor) -> Tensor {
        self.model.forward(inputs, self.device)
    }
    
    pub fn predict_batch(self: Self, inputs: Array[Tensor]) -> Array[Tensor] {
        inputs.chunks(self.batch_size)
            .map(|batch| self.predict(stack(batch)))
    }
    
    pub fn predict_with_uncertainty(self: Self, inputs: Tensor,
                                   num_samples: Int) -> Object {
        // Monte Carlo Dropout for uncertainty estimation
        let mut predictions = array()
        
        for _ in range(num_samples) {
            predictions.push(self.model.forward(inputs, self.device))
        }
        
        Object {
            mean: predictions.mean(),
            std: predictions.std(),
            samples: predictions
        }
    }
}
```

---

## IMPLEMENTATION CHECKLIST

### Phase 1 (Weeks 1-2)
- [ ] ModelBuilder with basic layers
- [ ] Compilation and model serialization
- [ ] Simple training loop
- [ ] CPU execution

### Phase 2 (Weeks 3-4)
- [ ] Auto-differentiation engine
- [ ] All layer types (50+)
- [ ] Optimizers (Adam, SGD, etc.)
- [ ] Metrics

### Phase 3 (Weeks 5-6)
- [ ] GPU training
- [ ] Distributed training
- [ ] Early stopping, checkpointing
- [ ] Learning rate scheduling

### Phase 4 (Weeks 7-8)
- [ ] JSON/YAML declarative API
- [ ] Model zoo and pretrained models
- [ ] Agent-driven design
- [ ] AutoML

---

**Document**: High-Level API Design  
**Version**: 1.0  
**Last Updated**: 2026-06-15
