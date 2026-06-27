# SYLVA Language Specification v1.0
## The Omnisystem ML/AI/Data Language

---

## 1. OVERVIEW

**SYLVA** replaces Python, Julia, R, JAX, TensorFlow. Features:
- Native tensors & automatic differentiation (AD)
- Lazy evaluation & computation graphs
- GPU-first design
- Statistical operations
- Neural network definitions

---

## 2. TENSOR OPERATIONS

### 2.1 Basic Tensors

```sylva
// Tensor creation
let x: Tensor<f32, [3, 4]> = Tensor::zeros()
let y: Tensor<f32, [3, 4]> = Tensor::ones()
let z: Tensor<f32, [5]> = Tensor::arange(0.0, 5.0)

// Tensor operations
let a: Tensor<f32, [3, 4]> = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]]
let b: Tensor<f32, [3, 4]> = a + 1.0
let c: Tensor<f32, [3, 4]> = a * 2.0
let d: Tensor<f32, [4, 3]> = a.transpose()

// Indexing
let elem: f32 = a[1, 2]  // 7.0
let row: Tensor<f32, [4]> = a[1, :]
let col: Tensor<f32, [3]> = a[:, 2]
```

### 2.2 Matrix Operations

```sylva
let a: Tensor<f32, [3, 3]> = identity_matrix()
let b: Tensor<f32, [3, 2]> = random_normal()

// Matrix multiplication
let c: Tensor<f32, [3, 2]> = matmul(a, b)

// Element-wise operations
let d: Tensor<f32, [3, 3]> = sin(a)
let e: Tensor<f32, [3, 3]> = relu(a)

// Reductions
let sum: f32 = sum(a)
let mean: f32 = mean(a)
let max_val: f32 = max(a)
let min_val: f32 = min(a)
```

---

## 3. AUTOMATIC DIFFERENTIATION

### 3.1 Gradient Computation

```sylva
// Define function to differentiate
fn loss(weights: Tensor<f32, [10, 5]>, input: Tensor<f32, [32, 10]>) -> f32 {
    let output: Tensor<f32, [32, 5]> = matmul(input, weights)
    let target: Tensor<f32, [32, 5]> = get_targets()
    
    let diff: Tensor<f32, [32, 5]> = output - target
    let squared: Tensor<f32, [32, 5]> = diff * diff
    
    return mean(squared)
}

// Compute gradients (automatic differentiation)
let grad_weights: Tensor<f32, [10, 5]> = grad(loss)(weights, input)

// Or use vjp for vector-Jacobian product
let (loss_val, vjp_fn) = vjp(loss, (weights, input))
let (grad_w, grad_i) = vjp_fn(1.0)
```

### 3.2 Gradient Descent Optimization

```sylva
fn train(model: NeuralNetwork, data: Dataset) -> void {
    let optimizer = Adam { lr: 0.001, beta1: 0.9, beta2: 0.999 }
    
    for epoch in 0..num_epochs {
        for (batch_x, batch_y) in data.batches() {
            // Forward pass
            let pred = model.forward(batch_x)
            let loss = cross_entropy(pred, batch_y)
            
            // Backward pass
            let grads = grad(loss)(model.parameters)
            
            // Update weights
            optimizer.update(model.parameters, grads)
        }
    }
}
```

---

## 4. NEURAL NETWORKS

### 4.1 Layer Definition

```sylva
// Dense layer
struct Dense {
    weights: Tensor<f32, [in_features, out_features]>,
    bias: Tensor<f32, [out_features]>,
    
    fn forward(input: Tensor<f32, [batch, in_features]>) 
        -> Tensor<f32, [batch, out_features]>
    {
        let output = matmul(input, weights)
        return output + bias
    }
}

// Convolutional layer
struct Conv2D {
    filters: Tensor<f32, [out_channels, in_channels, kernel_h, kernel_w]>,
    bias: Tensor<f32, [out_channels]>,
    stride: (i32, i32),
    padding: (i32, i32),
    
    fn forward(input: Tensor<f32, [batch, in_channels, height, width]>)
        -> Tensor<f32, [batch, out_channels, out_height, out_width]>
    {
        return conv2d(input, filters, bias, stride, padding)
    }
}
```

### 4.2 Model Definition

```sylva
struct MLP {
    layer1: Dense { in: 784, out: 128 },
    layer2: Dense { in: 128, out: 64 },
    layer3: Dense { in: 64, out: 10 },
    
    fn forward(input: Tensor<f32, [batch, 784]>) -> Tensor<f32, [batch, 10]> {
        let h1 = relu(layer1.forward(input))
        let h2 = relu(layer2.forward(h1))
        let output = softmax(layer3.forward(h2))
        return output
    }
}

struct CNN {
    conv1: Conv2D { in: 3, out: 32, kernel: (3, 3) },
    pool1: MaxPool2D { kernel: (2, 2) },
    conv2: Conv2D { in: 32, out: 64, kernel: (3, 3) },
    pool2: MaxPool2D { kernel: (2, 2) },
    fc: Dense { in: 64 * 7 * 7, out: 10 },
    
    fn forward(input: Tensor<f32, [batch, 3, 28, 28]>) -> Tensor<f32, [batch, 10]> {
        let x = relu(conv1.forward(input))
        let x = pool1.forward(x)
        let x = relu(conv2.forward(x))
        let x = pool2.forward(x)
        let x = x.reshape([-1, 64 * 7 * 7])
        let out = softmax(fc.forward(x))
        return out
    }
}
```

---

## 5. DATA OPERATIONS

### 5.1 DataFrame

```sylva
// Create DataFrame
let df = DataFrame {
    "name": Column<string>(["Alice", "Bob", "Charlie"]),
    "age": Column<i32>([25, 30, 35]),
    "salary": Column<f64>([50000.0, 60000.0, 75000.0])
}

// Selection
let names = df["name"]
let first_row = df[0, :]
let subset = df[df["age"] > 28, :]

// Aggregation
let mean_salary = df["salary"].mean()
let age_groups = df.group_by("age").count()

// Operations
let df2 = df.filter(|row| row["salary"] > 55000.0)
let df3 = df.map(|row| {
    return {
        name: row["name"],
        years_until_retirement: 65 - row["age"]
    }
})
```

### 5.2 Statistics

```sylva
// Distributions
let norm = Normal { mean: 0.0, std: 1.0 }
let samples = norm.sample(shape: [1000, 10])

// Probability density
let x = Tensor::linspace(-4.0, 4.0, 1000)
let pdf = norm.log_prob(x)

// Statistical tests
let t_statistic = t_test(sample1, sample2)
let p_value = chi_square_test(observed, expected)
```

---

## 6. LAZY EVALUATION & GRAPHS

### 6.1 Computation Graph

```sylva
// Operations are recorded in computation graph (not immediately executed)
let graph = Graph::new()

with graph {
    let x = Placeholder<f32>(shape: [32, 10])
    let w = Variable<f32>(Tensor::random_normal([10, 5]))
    let b = Variable<f32>(Tensor::zeros([5]))
    
    let output = matmul(x, w) + b
    let loss = mean_squared_error(output, targets)
    
    // Backward pass
    let grads = graph.gradient(loss, [w, b])
}

// Execute graph
let session = Session::new()
let result = session.run(output, {x: input_data})
```

### 6.2 JIT Compilation

```sylva
@jit
fn neural_network_forward(weights: [Tensor], inputs: Tensor) -> Tensor {
    let h1 = relu(matmul(inputs, weights[0]) + weights[1])
    let h2 = relu(matmul(h1, weights[2]) + weights[3])
    return matmul(h2, weights[4]) + weights[5]
}

// First call: JIT compiles to GPU/CPU optimized code
let output = neural_network_forward(weights, batch_data)

// Subsequent calls: Use compiled code (much faster)
let output2 = neural_network_forward(weights, batch_data2)
```

---

## 7. EXAMPLE: NEURAL NETWORK

```sylva
struct SimpleNN {
    w1: Tensor<f32, [10, 5]>,
    w2: Tensor<f32, [5, 1]>,
    
    fn forward(x: Tensor<f32, [batch, 10]>) -> Tensor<f32, [batch, 1]> {
        let h = relu(matmul(x, w1))
        return sigmoid(matmul(h, w2))
    }
}

fn train_network(model: &mut SimpleNN, data: [(Tensor, Tensor)]) -> void {
    let opt = Adam { lr: 0.01 }
    
    for epoch in 0..100 {
        let mut total_loss = 0.0
        
        for (x, y) in data {
            // Forward
            let pred = model.forward(x)
            let loss = mean((pred - y) * (pred - y))
            
            // Backward
            let grads = grad(loss)(model.parameters)
            
            // Update
            opt.apply_gradients(model.parameters, grads)
            
            total_loss += loss
        }
        
        if epoch % 10 == 0 {
            println("Epoch {}: loss = {}", epoch, total_loss / data.len())
        }
    }
}
```

---

This specification enables SYLVA to be the standard for machine learning and scientific computing.
