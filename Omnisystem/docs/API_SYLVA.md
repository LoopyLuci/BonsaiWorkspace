# SYLVA Runtime API Reference

**Complete API reference for machine learning and tensor operations**

---

## Module Overview

The SYLVA runtime provides:
- **Tensors**: N-dimensional arrays with efficient operations
- **Neural Networks**: Layer-based model building
- **Optimizers**: Gradient descent variants with learning rate scheduling
- **Loss Functions**: Regression and classification objectives
- **Datasets**: Data loading with augmentation

---

## Core Types

### Tensor<T>

**N-dimensional array for numerical computing**

```rust
pub struct Tensor<T> {
    data: Vec<T>,
    shape: Vec<usize>,
    strides: Vec<usize>,
}

impl<T> Tensor<T> {
    // Creation
    pub fn zeros(shape: &[usize]) -> Self
    pub fn ones(shape: &[usize]) -> Self
    pub fn random(shape: &[usize]) -> Self
    pub fn randn(shape: &[usize]) -> Self  // Normal distribution
    pub fn from_vec(data: Vec<T>, shape: Vec<usize>) -> Result<Self>
    pub fn eye(n: usize) -> Self  // Identity matrix
    pub fn arange(start: T, end: T, step: T) -> Self
    pub fn linspace(start: T, end: T, steps: usize) -> Self
    
    // Shape operations
    pub fn shape(&self) -> &[usize]
    pub fn reshape(&self, shape: &[usize]) -> Result<Self>
    pub fn flatten(&self) -> Self
    pub fn transpose(&self) -> Result<Self>
    pub fn permute(&self, axes: &[usize]) -> Result<Self>
    
    // Indexing
    pub fn get(&self, indices: &[usize]) -> Result<T>
    pub fn set(&mut self, indices: &[usize], value: T) -> Result<()>
    pub fn slice(&self, ranges: &[(usize, usize)]) -> Result<Self>
    
    // Statistics
    pub fn sum(&self) -> T
    pub fn mean(&self) -> T
    pub fn std(&self) -> T
    pub fn var(&self) -> T
    pub fn min(&self) -> T
    pub fn max(&self) -> T
    pub fn argmin(&self) -> usize
    pub fn argmax(&self) -> usize
}
```

**Example:**
```rust
let tensor = Tensor::random([3, 4, 5])
let mean = tensor.mean()
let reshaped = tensor.reshape(&[12, 5])?
```

---

### Dense Layer

**Fully-connected neural network layer**

```rust
pub struct Dense {
    weights: Tensor<f32>,
    bias: Tensor<f32>,
    input_size: usize,
    output_size: usize,
}

impl Dense {
    pub fn new(input_size: usize, output_size: usize) -> Self
    pub fn forward(&self, input: &Tensor<f32>) -> Result<Tensor<f32>>
    pub fn backward(&mut self, grad_output: &Tensor<f32>) -> Result<Tensor<f32>>
    pub fn get_weights(&self) -> &Tensor<f32>
    pub fn get_bias(&self) -> &Tensor<f32>
    pub fn set_weights(&mut self, weights: Tensor<f32>)
    pub fn set_bias(&mut self, bias: Tensor<f32>)
}
```

**Example:**
```rust
let layer = Dense::new(784, 128)
let output = layer.forward(&input)?  // [batch_size, 128]
```

---

### Conv2d Layer

**2D convolutional layer**

```rust
pub struct Conv2d {
    filters: Tensor<f32>,
    bias: Tensor<f32>,
    in_channels: usize,
    out_channels: usize,
    kernel_size: (usize, usize),
    stride: usize,
    padding: usize,
}

impl Conv2d {
    pub fn new(
        in_channels: usize,
        out_channels: usize,
        kernel_size: (usize, usize)
    ) -> Self
    
    pub fn with_stride(mut self, stride: usize) -> Self
    pub fn with_padding(mut self, padding: usize) -> Self
    pub fn forward(&self, input: &Tensor<f32>) -> Result<Tensor<f32>>
    pub fn backward(&mut self, grad_output: &Tensor<f32>) -> Result<Tensor<f32>>
}
```

---

### Activation Functions

**Non-linear activation layers**

```rust
pub mod activations {
    pub fn relu(x: &Tensor<f32>) -> Tensor<f32>
    pub fn sigmoid(x: &Tensor<f32>) -> Tensor<f32>
    pub fn tanh(x: &Tensor<f32>) -> Tensor<f32>
    pub fn softmax(x: &Tensor<f32>) -> Tensor<f32>
    pub fn gelu(x: &Tensor<f32>) -> Tensor<f32>
    pub fn swish(x: &Tensor<f32>) -> Tensor<f32>
    pub fn elu(x: &Tensor<f32>, alpha: f32) -> Tensor<f32>
    pub fn leaky_relu(x: &Tensor<f32>, alpha: f32) -> Tensor<f32>
}
```

**Example:**
```rust
let x = Tensor::randn([batch_size, features])
let activated = activations::relu(&x)
```

---

### Loss Functions

**Training objectives**

```rust
pub mod loss {
    // Regression
    pub fn mse_loss(pred: &Tensor<f32>, target: &Tensor<f32>) -> f32
    pub fn mae_loss(pred: &Tensor<f32>, target: &Tensor<f32>) -> f32
    pub fn huber_loss(pred: &Tensor<f32>, target: &Tensor<f32>, delta: f32) -> f32
    
    // Classification
    pub fn cross_entropy(pred: &Tensor<f32>, target: &Tensor<f32>) -> f32
    pub fn bce_loss(pred: &Tensor<f32>, target: &Tensor<f32>) -> f32
    
    // Distance
    pub fn kl_divergence(p: &Tensor<f32>, q: &Tensor<f32>) -> f32
    pub fn cosine_similarity(a: &Tensor<f32>, b: &Tensor<f32>) -> f32
}
```

**Example:**
```rust
let pred = model.forward(&input)?
let loss = loss::cross_entropy(&pred, &target)
```

---

### Adam Optimizer

**Adaptive moment estimation optimizer**

```rust
pub struct Adam {
    learning_rate: f32,
    beta1: f32,       // Momentum decay
    beta2: f32,       // RMSprop decay
    epsilon: f32,
    m: Vec<Tensor<f32>>,  // First moment
    v: Vec<Tensor<f32>>,  // Second moment
    t: usize,              // Timestep
}

impl Adam {
    pub fn new(learning_rate: f32) -> Self
    pub fn with_beta1(mut self, beta1: f32) -> Self
    pub fn with_beta2(mut self, beta2: f32) -> Self
    pub fn with_epsilon(mut self, epsilon: f32) -> Self
    
    pub fn step(&mut self, parameters: &mut [&mut Tensor<f32>])
    pub fn zero_grad(&mut self)
    pub fn state_dict(&self) -> OptimizerState
    pub fn load_state_dict(&mut self, state: OptimizerState)
}
```

**Example:**
```rust
let mut optimizer = Adam::new(0.001)
    .with_beta1(0.9)
    .with_beta2(0.999)

for epoch in 0..num_epochs {
    loss = train_step(&mut model)?
    optimizer.step(model.parameters())
    optimizer.zero_grad()
}
```

---

### Sequential Model

**Stack layers sequentially**

```rust
pub struct Sequential {
    layers: Vec<Box<dyn Layer>>,
}

impl Sequential {
    pub fn new() -> Self
    pub fn add<L: Layer + 'static>(&mut self, layer: L) -> &mut Self
    pub fn forward(&self, input: &Tensor<f32>) -> Result<Tensor<f32>>
    pub fn backward(&mut self, grad_output: &Tensor<f32>) -> Result<()>
    pub fn parameters(&mut self) -> Vec<&mut Tensor<f32>>
    pub fn save(&self, path: &str) -> Result<()>
    pub fn load(path: &str) -> Result<Self>
}
```

**Example:**
```rust
let mut model = Sequential::new()
    .add(Dense::new(784, 256))
    .add(Dense::new(256, 128))
    .add(Dense::new(128, 10))
```

---

### DataLoader

**Batch data loading with augmentation**

```rust
pub struct DataLoader {
    data: Vec<(Tensor<f32>, Tensor<f32>)>,
    batch_size: usize,
    shuffle: bool,
    num_workers: usize,
}

impl DataLoader {
    pub fn new(data: Vec<(Tensor<f32>, Tensor<f32>)>) -> Self
    pub fn with_batch_size(mut self, size: usize) -> Self
    pub fn with_shuffle(mut self, shuffle: bool) -> Self
    pub fn with_num_workers(mut self, workers: usize) -> Self
    pub fn with_augmentation(mut self, aug: DataAugmentation) -> Self
    pub fn into_iter(self) -> DataLoaderIter
    pub fn len(&self) -> usize
}

pub struct Batch {
    pub x: Tensor<f32>,
    pub y: Tensor<f32>,
}
```

**Example:**
```rust
let loader = DataLoader::new(train_data)
    .with_batch_size(32)
    .with_shuffle(true)

for batch in loader {
    let predictions = model.forward(&batch.x)?
    let loss = loss::cross_entropy(&predictions, &batch.y)
}
```

---

## Error Types

### TensorError

**Tensor operation errors**

```rust
pub enum TensorError {
    ShapeMismatch { expected: Vec<usize>, got: Vec<usize> },
    IndexOutOfBounds { index: Vec<usize>, shape: Vec<usize> },
    InvalidReshape { from: Vec<usize>, to: Vec<usize> },
    NotBroadcastable { shapes: Vec<Vec<usize>> },
}
```

### LayerError

**Neural network layer errors**

```rust
pub enum LayerError {
    InvalidInputShape { expected: usize, got: usize },
    NotInitialized,
    GradientNotComputed,
    WeightShapeMismatch,
}
```

### OptimizationError

**Training errors**

```rust
pub enum OptimizationError {
    DivergingGradients,
    NaNDetected,
    InsufficientData { required: usize, got: usize },
    NoConvergence,
}
```

---

## Usage Patterns

### Training Loop

```rust
let mut model = Sequential::new()
    .add(Dense::new(784, 256))
    .add(Dense::new(256, 10))

let mut optimizer = Adam::new(0.001)
let loader = DataLoader::new(train_data)
    .with_batch_size(32)
    .with_shuffle(true)

for epoch in 0..num_epochs {
    let mut epoch_loss = 0.0
    let mut count = 0
    
    for batch in &loader {
        let pred = model.forward(&batch.x)?
        let loss = loss::cross_entropy(&pred, &batch.y)
        
        model.backward(&loss)?
        optimizer.step(model.parameters())
        optimizer.zero_grad()
        
        epoch_loss += loss
        count += 1
    }
    
    println!("Epoch {}: loss = {:.4}", epoch, epoch_loss / count as f32)
}
```

### Model Evaluation

```rust
let mut correct = 0
let mut total = 0

for batch in &test_loader {
    let pred = model.forward(&batch.x)?
    let predictions = pred.argmax()
    let targets = batch.y.argmax()
    
    for (p, t) in predictions.iter().zip(targets.iter()) {
        if p == t { correct += 1 }
        total += 1
    }
}

let accuracy = correct as f32 / total as f32
println!("Accuracy: {:.2}%", accuracy * 100.0)
```

### Transfer Learning

```rust
// Load pretrained
let mut model = Sequential::load("resnet50.bin")?

// Freeze early layers
for i in 0..10 {
    model.layers[i].freeze()
}

// Add new head
model.add(Dense::new(2048, num_classes))

// Fine-tune
let fine_tune_loader = DataLoader::new(new_data)
    .with_batch_size(32)

for batch in &fine_tune_loader {
    let pred = model.forward(&batch.x)?
    let loss = loss::cross_entropy(&pred, &batch.y)
    model.backward(&loss)?
    optimizer.step(model.parameters())
}
```

---

## Helper Functions

### Data Augmentation

```rust
pub struct DataAugmentation {
    transforms: Vec<Box<dyn Transform>>,
}

impl DataAugmentation {
    pub fn new() -> Self
    pub fn add_random_flip(mut self, p: f32, horizontal: bool) -> Self
    pub fn add_random_rotation(mut self, max_angle: f32) -> Self
    pub fn add_random_crop(mut self, size: usize) -> Self
    pub fn add_random_noise(mut self, std: f32) -> Self
    pub fn apply(&self, tensor: &Tensor<f32>) -> Tensor<f32>
}
```

### Model Saving/Loading

```rust
impl Sequential {
    pub fn save(&self, path: &str) -> Result<()>
    pub fn load(path: &str) -> Result<Self>
    pub fn save_state_dict(&self, path: &str) -> Result<()>
    pub fn load_state_dict(&mut self, path: &str) -> Result<()>
    pub fn export_onnx(&self, path: &str) -> Result<()>
    pub fn export_torchscript(&self, path: &str) -> Result<()>
}
```

---

## Constants

### Common Hyperparameters

```rust
pub const ADAM_BETA1: f32 = 0.9
pub const ADAM_BETA2: f32 = 0.999
pub const ADAM_EPSILON: f32 = 1e-8

pub const SGD_MOMENTUM: f32 = 0.9
pub const SGD_DAMPENING: f32 = 0.0

pub const RELU_NEGATIVE_SLOPE: f32 = 0.0
pub const LEAKY_RELU_SLOPE: f32 = 0.01
pub const ELU_ALPHA: f32 = 1.0
```

---

## Examples

### MNIST Classification

```rust
use sylva::nn::*
use sylva::optim::*
use sylva::dataset::*

let train_data = Dataset::load_mnist("train")?
let test_data = Dataset::load_mnist("test")?

let mut model = Sequential::new()
    .add(Dense::new(784, 256))
    .add(Dense::new(256, 128))
    .add(Dense::new(128, 10))

let mut optimizer = Adam::new(0.001)
let train_loader = DataLoader::new(train_data)
    .with_batch_size(32)
    .with_shuffle(true)

for epoch in 0..10 {
    for batch in &train_loader {
        let pred = model.forward(&batch.x)?
        let loss = loss::cross_entropy(&pred, &batch.y)
        model.backward(&loss)?
        optimizer.step(model.parameters())
        optimizer.zero_grad()
    }
}

model.save("mnist_model.bin")?
```

### Image Classification

```rust
let mut model = Sequential::new()
    .add(Conv2d::new(3, 32, (3, 3)).with_padding(1))
    .add(Conv2d::new(32, 64, (3, 3)).with_padding(1))
    .add(Dense::new(64 * 8 * 8, 256))
    .add(Dense::new(256, num_classes))

// Train with ImageNet data
let loader = DataLoader::new(imagenet_data)
    .with_batch_size(128)
    .with_augmentation(
        DataAugmentation::new()
            .add_random_flip(0.5, true)
            .add_random_rotation(15.0)
            .add_random_crop(224)
    )
```

---

## Testing

### Unit Tests

```rust
#[test]
fn test_tensor_shape() {
    let t = Tensor::random([3, 4, 5])
    assert_eq!(t.shape(), &[3, 4, 5])
}

#[test]
fn test_dense_forward() {
    let layer = Dense::new(10, 5)
    let input = Tensor::randn([2, 10])
    let output = layer.forward(&input).unwrap()
    assert_eq!(output.shape(), &[2, 5])
}

#[test]
fn test_loss_function() {
    let pred = Tensor::ones([10])
    let target = Tensor::ones([10])
    let loss = loss::mse_loss(&pred, &target)
    assert!(loss < 1e-6)
}
```

---

## Performance Notes

- **Tensor operations** are optimized with SIMD where available
- **Batch processing** essential for GPU efficiency
- **Mixed precision** (FP32/FP16) reduces memory by 2x
- **Gradient accumulation** enables larger effective batch sizes
- **Data pipeline** should run in parallel with training

---

## See Also
- [SYLVA_LANGUAGE_GUIDE.md](SYLVA_LANGUAGE_GUIDE.md) - Language tutorial
- [TUTORIAL_ML_AI.md](TUTORIAL_ML_AI.md) - Complete ML example
- [SYLVA_LANGUAGE_SPECIFICATION.md](SYLVA_LANGUAGE_SPECIFICATION.md) - Formal spec

---

**Last Updated**: 2026-06-15
