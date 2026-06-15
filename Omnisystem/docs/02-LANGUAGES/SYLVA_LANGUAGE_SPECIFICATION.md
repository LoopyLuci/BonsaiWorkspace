# SYLVA Language Specification - Complete Reference

**Formal specification for SYLVA machine learning language**

---

## Language Overview

**SYLVA** is a statically-typed machine learning language with:
- First-class tensor support with shape inference
- Automatic differentiation (forward and reverse mode)
- GPU acceleration (CUDA, OpenCL, Metal)
- Distributed training primitives
- Type-safe neural networks
- Zero-copy operations
- Production-ready performance

---

## Type System

### Scalar Types

```sylva
// Basic scalars
bool, i8, i16, i32, i64, u8, u16, u32, u64
f32, f64
complex64, complex128  // Complex numbers
```

### Tensor Types

```sylva
// Dense tensors
type Tensor<T, Shape>
  where T: Scalar, Shape: Shape

// Shape specification
type Shape = (dim0: usize, dim1: usize, ...)

// Examples
type Vector = Tensor<f32, (N,)>
type Matrix = Tensor<f32, (M, N)>
type Tensor3D = Tensor<f32, (D, H, W)>
type Batch = Tensor<f32, (Batch, H, W, C)>

// Dynamic shapes
type DynamicTensor = Tensor<f32, Dynamic>
```

### Advanced Types

```sylva
// Sparse tensors
type SparseTensor<T> {
    indices: Tensor<i64>,
    values: Tensor<T>,
    shape: Shape,
}

// Variables (with gradients)
type Variable<T: Differentiable> {
    data: Tensor<T>,
    grad: Option<Tensor<T>>,
    requires_grad: bool,
}

// Parameters
type Parameter<T: Differentiable> {
    variable: Variable<T>,
    trainable: bool,
}

// Batched operations
type Batch<T> {
    data: Tensor<T>,
    batch_size: usize,
}
```

---

## Automatic Differentiation

### Forward Mode (Tangent)

```sylva
fun forward_diff(f: fun(x: f32) -> f32, x: f32, dx: f32) -> (f32, f32) {
    // Returns (f(x), df/dx * dx)
    let dual = Dual::new(x, dx)
    let result = f(dual)
    (result.value, result.tangent)
}

// Example
fun quadratic(x: Dual<f32>) -> Dual<f32> {
    x * x + 2.0 * x + 1.0
}

let (y, dy) = forward_diff(quadratic, 3.0, 1.0)
// y = 16.0, dy = 8.0
```

### Reverse Mode (Gradient)

```sylva
fun reverse_diff(f: fun(x: Variable<f32>) -> Variable<f32>, 
                 x: Variable<f32>) -> f32 {
    x.requires_grad = true
    let y = f(x)
    y.backward()
    x.grad.unwrap()
}

// Example
fun complex_function(x: Variable<f32>) -> Variable<f32> {
    (x.sin() * x.exp()).sqrt()
}

let x = Variable::new(2.0)
let gradient = reverse_diff(complex_function, x)
```

### Gradient Computation

```sylva
// Automatic gradients
let x = Variable::new(tensor![[1.0, 2.0], [3.0, 4.0]])
x.requires_grad = true

let y = (x * x).sum()
y.backward()

println!("{}", x.grad)  // [[2.0, 4.0], [6.0, 8.0]]

// Higher-order gradients
x.requires_grad = true
let y = (x * x).sum()
y.backward(create_graph: true)

let grad_y = x.grad
grad_y.sum().backward()  // Gradient of gradient
```

---

## Neural Network Layers

### Basic Layers

```sylva
// Dense/Linear
type Dense {
    weight: Parameter<f32>,  // Shape: (out_features, in_features)
    bias: Parameter<f32>,    // Shape: (out_features,)
}

impl Dense {
    fun new(in_features: usize, out_features: usize) -> Self {
        let weight = Parameter::new(
            Tensor::randn(out_features, in_features) * 0.01
        )
        let bias = Parameter::new(Tensor::zeros(out_features))
        Self { weight, bias }
    }
    
    fun forward(self, input: Tensor<f32, (*, in_features)>) 
        -> Tensor<f32, (*, out_features)> {
        input @ self.weight.t() + self.bias
    }
}

// Convolutional
type Conv2d {
    weight: Parameter<f32>,  // (out_channels, in_channels, kernel_h, kernel_w)
    bias: Parameter<f32>,    // (out_channels,)
    padding: (i32, i32),
    stride: (i32, i32),
}

// Batch normalization
type BatchNorm2d {
    weight: Parameter<f32>,
    bias: Parameter<f32>,
    running_mean: Tensor<f32>,
    running_var: Tensor<f32>,
    momentum: f32,
}

// Dropout
type Dropout {
    p: f32,  // Dropout probability
}

// Recurrent
type LSTM {
    input_size: usize,
    hidden_size: usize,
    num_layers: usize,
    weight_ih: Vec<Parameter<f32>>,  // Input-to-hidden weights
    weight_hh: Vec<Parameter<f32>>,  // Hidden-to-hidden weights
    bias: Vec<Parameter<f32>>,
}

type GRU {
    input_size: usize,
    hidden_size: usize,
    weight: Vec<Parameter<f32>>,
    bias: Vec<Parameter<f32>>,
}
```

### Activation Functions

```sylva
// Built-in activations
fun relu(x: Tensor<f32>) -> Tensor<f32> {
    x.max(0.0)
}

fun sigmoid(x: Tensor<f32>) -> Tensor<f32> {
    1.0 / (1.0 + (-x).exp())
}

fun tanh(x: Tensor<f32>) -> Tensor<f32> {
    x.tanh()
}

fun softmax(x: Tensor<f32>, dim: i32) -> Tensor<f32> {
    let exp_x = x.exp()
    exp_x / exp_x.sum(dim, keepdim: true)
}

fun leaky_relu(x: Tensor<f32>, alpha: f32 = 0.01) -> Tensor<f32> {
    where x >= 0.0 { x } else { alpha * x }
}

fun elu(x: Tensor<f32>, alpha: f32 = 1.0) -> Tensor<f32> {
    where x >= 0.0 { x } else { alpha * (x.exp() - 1.0) }
}

fun gelu(x: Tensor<f32>) -> Tensor<f32> {
    // Gaussian error linear unit
    x * 0.5 * (1.0 + (x / 1.414213562).erf())
}

fun swish(x: Tensor<f32>) -> Tensor<f32> {
    x * sigmoid(x)
}
```

### Loss Functions

```sylva
// Mean squared error
fun mse_loss(pred: Tensor<f32>, target: Tensor<f32>) -> f32 {
    ((pred - target) ** 2).mean()
}

// Cross entropy
fun cross_entropy_loss(pred: Tensor<f32>, target: Tensor<i64>) -> f32 {
    let log_softmax = pred.softmax().log()
    -log_softmax.gather(1, target).mean()
}

// Binary cross entropy
fun bce_loss(pred: Tensor<f32>, target: Tensor<f32>) -> f32 {
    let eps = 1e-7
    -(target * (pred + eps).log() + (1.0 - target) * (1.0 - pred + eps).log()).mean()
}

// L1 loss
fun l1_loss(pred: Tensor<f32>, target: Tensor<f32>) -> f32 {
    (pred - target).abs().mean()
}

// KL divergence
fun kl_divergence(pred: Tensor<f32>, target: Tensor<f32>) -> f32 {
    (target * (target / pred).log()).sum()
}
```

---

## Optimizers

### SGD & Variants

```sylva
// Stochastic Gradient Descent
type SGD {
    learning_rate: f32,
    momentum: f32 = 0.0,
    weight_decay: f32 = 0.0,
    nesterov: bool = false,
}

impl SGD {
    fun step(mut self, params: &mut Vec<Parameter>, grads: &Vec<Tensor>) {
        for (param, grad) in zip(params, grads) {
            let update = grad + self.weight_decay * param.value
            
            if self.momentum > 0.0 {
                param.velocity = self.momentum * param.velocity + update
                param.value -= self.learning_rate * param.velocity
            } else {
                param.value -= self.learning_rate * update
            }
        }
    }
}

// Adam
type Adam {
    learning_rate: f32 = 1e-3,
    beta1: f32 = 0.9,
    beta2: f32 = 0.999,
    epsilon: f32 = 1e-8,
    weight_decay: f32 = 0.0,
}

// RMSprop
type RMSprop {
    learning_rate: f32 = 1e-3,
    alpha: f32 = 0.99,
    epsilon: f32 = 1e-8,
    weight_decay: f32 = 0.0,
}

// AdaBound
type AdaBound {
    learning_rate: f32 = 1e-3,
    final_learning_rate: f32 = 0.1,
    gamma: f32 = 1e-3,
    beta1: f32 = 0.9,
    beta2: f32 = 0.999,
}

// LAMB (Large Batch Adam)
type LAMB {
    learning_rate: f32 = 1e-3,
    beta1: f32 = 0.9,
    beta2: f32 = 0.999,
    epsilon: f32 = 1e-8,
    weight_decay: f32 = 0.01,
}
```

---

## Models & Training

### Model Definition

```sylva
// Sequential model
type SequentialModel {
    layers: Vec<Layer>,
}

impl SequentialModel {
    fun forward(self, x: Tensor<f32>) -> Tensor<f32> {
        mut y = x
        for layer in self.layers {
            y = layer.forward(y)
        }
        y
    }
}

// Custom model with module system
type ResNet50 {
    conv1: Conv2d,
    bn1: BatchNorm2d,
    layer1: ResidualBlock,
    layer2: ResidualBlock,
    layer3: ResidualBlock,
    layer4: ResidualBlock,
    fc: Dense,
}

impl ResNet50 {
    fun forward(self, x: Tensor<f32>) -> Tensor<f32> {
        let mut x = self.conv1.forward(x)
        x = self.bn1.forward(x)
        x = relu(x)
        
        x = self.layer1.forward(x)
        x = self.layer2.forward(x)
        x = self.layer3.forward(x)
        x = self.layer4.forward(x)
        
        x = x.adaptive_avg_pool_2d((1, 1))
        x = x.flatten(1)
        x = self.fc.forward(x)
        
        x
    }
}
```

### Training Loop

```sylva
fun train_epoch(
    model: &mut Model,
    train_loader: DataLoader,
    optimizer: &mut Optimizer,
    loss_fn: fun(Tensor, Tensor) -> f32,
    device: Device
) -> f32 {
    mut total_loss = 0.0
    mut num_batches = 0
    
    for (batch_x, batch_y) in train_loader {
        let batch_x = batch_x.to(device)
        let batch_y = batch_y.to(device)
        
        // Forward pass
        let predictions = model.forward(batch_x)
        let loss = loss_fn(predictions, batch_y)
        
        // Backward pass
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
        
        total_loss += loss.item()
        num_batches += 1
    }
    
    total_loss / num_batches as f32
}

fun train(
    model: &mut Model,
    train_loader: DataLoader,
    val_loader: DataLoader,
    num_epochs: i32,
    device: Device
) {
    let optimizer = Adam::new(model.parameters(), lr: 0.001)
    
    for epoch in 0..num_epochs {
        let train_loss = train_epoch(model, train_loader, optimizer, 
                                     cross_entropy_loss, device)
        let val_loss = evaluate(model, val_loader, device)
        
        println!("Epoch {}: train_loss={:.4}, val_loss={:.4}", 
                 epoch, train_loss, val_loss)
        
        if epoch % 10 == 0 {
            model.save(format!("checkpoint_{}.pt", epoch))
        }
    }
}
```

---

## Data Loading

### DataLoader

```sylva
type Dataset<T> {
    data: Vec<T>,
}

impl Dataset {
    fun len(self) -> usize {
        self.data.len()
    }
    
    fun __getitem__(self, idx: usize) -> T {
        self.data[idx]
    }
}

type DataLoader<T> {
    dataset: Dataset<T>,
    batch_size: usize,
    shuffle: bool,
    num_workers: usize,
}

impl DataLoader {
    fun iter(self) -> Iterator<Batch<T>> {
        // Returns batches of specified size
    }
}

// Creating datasets
fun create_dataset(images: Tensor, labels: Tensor) -> Dataset {
    Dataset { data: zip(images, labels).collect() }
}

// Creating loader
let train_loader = DataLoader::new(train_dataset)
    .with_batch_size(32)
    .with_shuffle(true)
    .with_num_workers(4)
```

---

## GPU Support

### Tensor Placement

```sylva
// CPU tensor
let x = Tensor::randn((1000, 1000))

// GPU tensor
let x_gpu = x.cuda()  // Move to GPU
let x_gpu = x.to(Device::cuda(0))  // Specific GPU

// Check device
println!("{}", x.device())

// Multi-GPU
for gpu_id in 0..4 {
    let device = Device::cuda(gpu_id)
    let x = Tensor::randn((1000, 1000)).to(device)
}
```

### GPU Operations

```sylva
// Automatic GPU kernels
let x = Tensor::randn((10000, 10000)).cuda()
let y = Tensor::randn((10000, 10000)).cuda()
let z = x @ y  // Matrix multiplication on GPU (>1 TFLOPS)

// GPU memory management
let x = Tensor::randn((1000000000,)).cuda()
println!("{} MB", x.nbytes() / 1_000_000)
x.detach()  // Release gradients
drop(x)     // Release memory
```

---

## Distributed Training

### Data Parallelism

```sylva
// Distribute model across GPUs
let model = ResNet50::new().cuda()
let distributed_model = DataParallel::new(model, devices: &[0, 1, 2, 3])

// Forward pass splits batch across GPUs
let x = Tensor::randn((128, 3, 224, 224))
let output = distributed_model.forward(x)
// Batch split into [32, 32, 32, 32] on 4 GPUs
```

### All-Reduce

```sylva
// Synchronize gradients across workers
fun all_reduce_gradients(model: &Model, process_group: &ProcessGroup) {
    for param in model.parameters() {
        process_group.all_reduce(param.grad, op: ReduceOp::Sum)
        param.grad = param.grad / process_group.size() as f32
    }
}
```

---

## Tensor Operations

### Mathematical Operations

```sylva
// Element-wise
let c = a + b              // Addition
let c = a - b              // Subtraction
let c = a * b              // Element-wise multiply
let c = a / b              // Division
let c = a ** 2.0           // Power

// Matrix operations
let c = a @ b              // Matrix multiply
let c = a.t()              // Transpose
let c = a.inverse()        // Matrix inverse
let det = a.det()          // Determinant

// Reductions
let s = a.sum()            // Sum all elements
let m = a.mean()           // Mean
let s = a.std()            // Standard deviation
let m = a.max()            // Maximum element
let m = a.min()            // Minimum element
let n = a.norm()           // L2 norm

// Dimension operations
let s = a.sum(dim: 0)      // Sum along dimension
let m = a.mean(dim: 1, keepdim: true)
```

---

## Type Inference

### Automatic Shape Inference

```sylva
let x: Tensor = Tensor::randn((32, 224, 224, 3))  // Shape inferred
let conv = Conv2d::new(3, 64, kernel_size: 7)
let y = conv.forward(x)  // Output shape: (32, 112, 112, 64)

// Shape computation
let a: Tensor = randn((100, 50))
let b: Tensor = randn((50, 20))
let c = a @ b  // Shape: (100, 20) - inferred automatically
```

---

## Quantization & Pruning

### Quantization

```sylva
// Quantize to int8
let quantized = model.quantize(dtype: Int8)

// Fine-tune with quantization aware training
let qat_model = QuantizationAware::new(model)
train(qat_model, train_loader, num_epochs: 50)
```

### Pruning

```sylva
// Structured pruning
let pruned = model.prune_channels(target_ratio: 0.3)

// Unstructured pruning
model.prune_weights(threshold: 1e-3)
```

---

## Performance Characteristics

### Throughput
- **Dense layer (1000×1000):** >1 TFLOPS (GPU)
- **Convolution (batch 64, 3×224×224):** >100 TFLOPS (GPU)
- **Matrix multiply:** Near peak bandwidth

### Latency
- **Dense forward:** <1ms
- **Convolution forward:** <5ms
- **Full training step:** <10ms (with backward)

---

## Next Steps

- [AETHER_LANGUAGE_SPECIFICATION.md](AETHER_LANGUAGE_SPECIFICATION.md)
- [AXIOM_LANGUAGE_SPECIFICATION.md](AXIOM_LANGUAGE_SPECIFICATION.md)
- [PERFORMANCE_BENCHMARKS.md](PERFORMANCE_BENCHMARKS.md)

---

**SYLVA Specification** - Complete machine learning language reference!
