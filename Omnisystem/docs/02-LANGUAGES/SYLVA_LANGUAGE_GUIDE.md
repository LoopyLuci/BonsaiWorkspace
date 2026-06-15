# SYLVA Language Guide - Machine Learning & AI

**High-performance ML/AI framework with automatic differentiation and neural networks**

---

## Table of Contents
1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Tensors](#tensors)
4. [Neural Networks](#neural-networks)
5. [Training Models](#training-models)
6. [Activation Functions](#activation-functions)
7. [Loss Functions](#loss-functions)
8. [Optimizers](#optimizers)
9. [Data Loading](#data-loading)
10. [Advanced Topics](#advanced-topics)

---

## Introduction

SYLVA is a machine learning language optimized for:
- **Performance**: GPU-accelerated tensor operations
- **Simplicity**: Intuitive ML operations
- **Flexibility**: Support for custom architectures
- **Production Ready**: Deploy trained models immediately

### Quick Facts
- **Execution**: JIT compiled with optimization passes
- **Automatic Differentiation**: Gradients computed automatically
- **GPU Support**: CUDA and OpenCL compatible
- **Standard Library**: 20+ ML/AI modules

---

## Getting Started

### Your First Neural Network

```sylva
// mnist.sy
use sylva::nn::*
use sylva::optim::*

fun main() {
    // Create network
    let mut model = Sequential::new()
    model.add(Dense::new(784, 128))
    model.add(Dense::new(128, 10))
    
    // Create optimizer
    let mut optimizer = Adam::new(0.001)
    
    // Train
    for epoch in 0..10 {
        let loss = train_epoch(&mut model, &train_data, &mut optimizer)
        println!("Epoch {}: loss = {}", epoch, loss)
    }
    
    // Evaluate
    let accuracy = evaluate(&model, &test_data)
    println!("Test Accuracy: {:.2}%", accuracy * 100.0)
}
```

### Running Your Model

```bash
omnisystem run mnist.sy
# Epoch 0: loss = 2.3045
# Epoch 1: loss = 1.8234
# ...
# Test Accuracy: 97.45%
```

---

## Tensors

### Creating Tensors

```sylva
use sylva::tensor::*

// Creation
let zeros = Tensor::zeros([2, 3, 4])
let ones = Tensor::ones([2, 3])
let random = Tensor::random([100, 50])
let randn = Tensor::randn([64, 128])  // Normal distribution

// From data
let data = [1.0, 2.0, 3.0, 4.0]
let tensor = Tensor::from_vec(data, [2, 2])?

// Special matrices
let identity = Tensor::eye(5)  // 5x5 identity
let range = Tensor::arange(0.0, 10.0, 1.0)
let linspace = Tensor::linspace(0.0, 1.0, 100)
```

### Tensor Operations

```sylva
// Shape operations
let original = Tensor::random([2, 3, 4])
let reshaped = original.reshape([6, 4])?
let flattened = original.flatten()
let transposed = original.transpose()?

// Indexing
let value = tensor.get(&[0, 1, 2])?
tensor.set(&[0, 1, 2], 5.0)?
let slice = tensor.slice(&[(0, 2), (0, 3)])?

// Statistics
let sum = tensor.sum()
let mean = tensor.mean()
let std = tensor.std()
let var = tensor.var()
let min = tensor.min()
let max = tensor.max()
```

### Tensor Arithmetic

```sylva
// Element-wise operations
let a = Tensor::random([2, 3])
let b = Tensor::random([2, 3])

let sum = &a + &b
let diff = &a - &b
let prod = &a * &b
let div = &a / &b

// Broadcasting
let vector = Tensor::random([3])
let matrix = Tensor::random([2, 3])
let broadcast = &matrix + &vector

// Reductions
let result = a.sum()      // Scalar
let result = a.mean()     // Scalar
let result = a.std()      // Scalar
```

---

## Neural Networks

### Building Models

```sylva
use sylva::nn::*

// Sequential API
let mut model = Sequential::new()
model.add(Dense::new(784, 256))
model.add(Dense::new(256, 128))
model.add(Dense::new(128, 10))

// Or using builder
let model = Sequential::new()
    .add(Dense::new(784, 256))
    .add(Dense::new(256, 128))
    .add(Dense::new(128, 10))
```

### Layers

```sylva
// Dense layer
let layer = Dense::new(input_size, output_size)
let output = layer.forward(&input)?

// Convolutional
let conv = Conv2d::new(
    in_channels: 3,
    out_channels: 32,
    kernel_size: (3, 3)
)

// LSTM
let lstm = LSTM::new(input_size, hidden_size)
let (output, (h, c)) = lstm.forward(&input, &hidden_state)?

// GRU
let gru = GRU::new(input_size, hidden_size)

// Attention
let attention = Attention::new(dim)

// Dropout
let dropout = Dropout::new(p: 0.5)

// BatchNorm
let batchnorm = BatchNorm::new(num_features)
```

### Forward Pass

```sylva
fun forward(&self, input: &Tensor) -> Result<Tensor> {
    let mut x = input.clone()
    
    for layer in &self.layers {
        x = layer.forward(&x)?
    }
    
    Ok(x)
}

// Usage
let input = Tensor::random([batch_size, input_dim])
let output = model.forward(&input)?
```

---

## Training Models

### Complete Training Loop

```sylva
fun train_epoch(
    model: &mut Sequential,
    train_loader: &DataLoader,
    optimizer: &mut Adam,
    loss_fn: impl Fn(&Tensor, &Tensor) -> f32
) -> f32 {
    let mut total_loss = 0.0
    let mut count = 0
    
    for batch in train_loader {
        // Forward pass
        let predictions = model.forward(&batch.x)?
        let loss = loss_fn(&predictions, &batch.y)
        
        // Backward pass
        model.backward(&loss)?
        
        // Optimization step
        optimizer.step(&model.parameters())
        optimizer.zero_grad()
        
        total_loss += loss
        count += 1
    }
    
    total_loss / count as f32
}

// Training loop
for epoch in 0..num_epochs {
    let loss = train_epoch(&mut model, &train_loader, &mut optimizer, loss_fn)
    
    // Validation
    let val_loss = evaluate(&model, &val_loader)
    println!("Epoch {}: train={:.4}, val={:.4}", epoch, loss, val_loss)
    
    // Early stopping
    if val_loss > best_loss * 1.05 {
        break
    }
    best_loss = val_loss
}
```

---

## Activation Functions

```sylva
use sylva::nn::activations::*

let x = Tensor::randn([batch_size, features])

// ReLU
let y = relu(&x)  // max(0, x)

// Sigmoid
let y = sigmoid(&x)  // 1 / (1 + exp(-x))

// Tanh
let y = tanh(&x)  // (exp(2x) - 1) / (exp(2x) + 1)

// Softmax
let y = softmax(&x)  // exp(x) / sum(exp(x))

// GELU
let y = gelu(&x)  // Gaussian Error Linear Unit

// Swish
let y = swish(&x)  // x * sigmoid(x)

// ELU
let y = elu(&x, alpha: 1.0)
```

---

## Loss Functions

```sylva
use sylva::nn::loss_functions::*

let predictions = model.forward(&inputs)?
let targets = get_targets()

// Regression losses
let mse = mse_loss(&predictions, &targets)
let mae = mae_loss(&predictions, &targets)
let huber = huber_loss(&predictions, &targets, delta: 1.0)

// Classification losses
let ce = cross_entropy(&predictions, &targets)
let bce = bce_loss(&predictions, &targets)

// Distance losses
let kld = kl_divergence(&predictions, &targets)
let cosine = cosine_similarity_loss(&predictions, &targets)
```

---

## Optimizers

### Adam Optimizer

```sylva
use sylva::optim::*

let mut optimizer = Adam::new(learning_rate: 0.001)

// With custom parameters
let mut optimizer = Adam::new(0.001)
    .with_beta1(0.9)
    .with_beta2(0.999)
    .with_epsilon(1e-8)

// Training step
for batch in train_loader {
    let loss = train_step(&mut model, &batch)?
    
    optimizer.step(model.parameters())
    optimizer.zero_grad()
}
```

### Other Optimizers

```sylva
// SGD with momentum
let mut optimizer = SGD::new(0.01)
    .with_momentum(0.9)

// RMSprop
let mut optimizer = RMSprop::new(0.001)
    .with_decay(0.99)

// AdaGrad
let mut optimizer = AdaGrad::new(0.01)

// AdamW (weight decay)
let mut optimizer = AdamW::new(0.001)
    .with_weight_decay(1e-4)

// Learning rate scheduling
let scheduler = CosineAnnealing::new(
    initial_lr: 0.1,
    min_lr: 0.0001,
    T_max: num_epochs
)
```

---

## Data Loading

### Creating DataLoaders

```sylva
use sylva::dataset::*

// Create dataset
let train_data = Dataset::from_files(
    images: "data/train_images.npy",
    labels: "data/train_labels.npy"
)?

// Create loader
let loader = DataLoader::new(train_data)
    .with_batch_size(32)
    .with_shuffle(true)
    .with_num_workers(4)

// Iterate
for batch in loader {
    let x = batch.features  // [batch_size, features]
    let y = batch.labels    // [batch_size]
    
    // Train on batch
    let loss = train_step(&mut model, &x, &y)?
}
```

### Data Augmentation

```sylva
let augmentation = DataAugmentation::new()
    .add(RandomFlip::new(probability: 0.5, horizontal: true))
    .add(RandomRotation::new(max_angle: 15.0))
    .add(RandomNoise::new(std: 0.1))
    .add(RandomCrop::new(size: 32))

let train_loader = DataLoader::new(train_data)
    .with_augmentation(augmentation)
    .with_batch_size(batch_size)
```

---

## Advanced Topics

### Custom Layers

```sylva
use sylva::nn::*

type CustomLayer {
    weight: Tensor,
    bias: Tensor,
}

impl Layer for CustomLayer {
    fun forward(&self, input: &Tensor) -> Result<Tensor> {
        let output = input.matmul(&self.weight)?
        Ok(&output + &self.bias)
    }
    
    fun backward(&mut self, grad_output: &Tensor) -> Result<Tensor> {
        // Compute gradients
        self.weight_grad = input.t().matmul(grad_output)?
        Ok(grad_output.matmul(&self.weight.t())?)
    }
}
```

### Transfer Learning

```sylva
// Load pretrained model
let mut model = load_pretrained("resnet50")?

// Freeze backbone
for layer in model.layers[..layer_idx].iter_mut() {
    layer.freeze()
}

// Add new head
model.add(Dense::new(2048, 1000))
model.add(Dense::new(1000, num_classes))

// Fine-tune on new data
for epoch in 0..num_epochs {
    train_epoch(&mut model, &train_loader, &mut optimizer)?
}
```

### Model Serialization

```sylva
// Save model
model.save("model.bin")?
model.save_state_dict("weights.bin")?

// Load model
let model = Sequential::load("model.bin")?
let state = load_state_dict("weights.bin")?
model.load_state_dict(&state)?

// Export for inference
model.export_onnx("model.onnx")?
model.export_torchscript("model.pt")?
```

---

## Complete Example: MNIST Classifier

```sylva
use sylva::nn::*
use sylva::optim::*
use sylva::dataset::*

fun create_model() -> Sequential {
    Sequential::new()
        .add(Dense::new(784, 256))
        .add(Dense::new(256, 128))
        .add(Dense::new(128, 10))
}

fun train() -> Result<()> {
    // Load data
    let train_data = Dataset::load_mnist("train")?
    let test_data = Dataset::load_mnist("test")?
    
    let train_loader = DataLoader::new(train_data)
        .with_batch_size(32)
        .with_shuffle(true)
    
    // Create model and optimizer
    let mut model = create_model()
    let mut optimizer = Adam::new(0.001)
    
    // Training loop
    for epoch in 0..10 {
        let mut total_loss = 0.0
        let mut count = 0
        
        for batch in &train_loader {
            // Forward
            let predictions = model.forward(&batch.x)?
            let loss = cross_entropy(&predictions, &batch.y)
            
            // Backward
            model.backward(&loss)?
            
            // Update
            optimizer.step(model.parameters())
            optimizer.zero_grad()
            
            total_loss += loss
            count += 1
        }
        
        let avg_loss = total_loss / count as f32
        
        // Evaluation
        let accuracy = evaluate(&model, &test_data)?
        println!("Epoch {}: loss={:.4}, accuracy={:.2}%",
            epoch, avg_loss, accuracy * 100.0)
    }
    
    // Save model
    model.save("mnist_model.bin")?
    
    Ok(())
}

fun main() -> Result<()> {
    train()
}
```

---

## Best Practices

✅ **DO**
- Use batching for performance
- Normalize input data
- Use appropriate learning rates
- Monitor validation loss
- Implement early stopping
- Save best models
- Use appropriate batch sizes
- Document model architecture

❌ **DON'T**
- Train on full dataset at once
- Use raw input data
- Ignore validation metrics
- Overfit without regularization
- Use same learning rate throughout
- Train for fixed epochs
- Forget to normalize batch norm
- Hardcode architecture

---

## Performance Tips

1. **Batch Processing** - Maximize GPU utilization
2. **Data Pipeline** - Asynchronous loading
3. **Mixed Precision** - FP32/FP16 training
4. **Gradient Accumulation** - Larger effective batches
5. **Distributed Training** - Multi-GPU/Multi-node
6. **Model Optimization** - Pruning, quantization

---

## Debugging

### Training Diagnostics

```sylva
// Check gradients
let grads = model.get_gradients()
for (i, grad) in grads.iter().enumerate() {
    let grad_norm = grad.norm()
    println!("Layer {} gradient norm: {}", i, grad_norm)
}

// Monitor loss
if loss.is_nan() {
    println!("NaN detected! Check learning rate")
}

// Validation checks
if val_loss > train_loss * 2.0 {
    println!("Possible overfitting")
}
```

---

## See Also
- [API_SYLVA.md](API_SYLVA.md) - Complete API reference
- [TUTORIAL_ML_AI.md](TUTORIAL_ML_AI.md) - Complete ML example
- [SYLVA_LANGUAGE_SPECIFICATION.md](SYLVA_LANGUAGE_SPECIFICATION.md) - Formal spec

---

**Next**: [TUTORIAL_ML_AI.md](TUTORIAL_ML_AI.md) - Train production models
