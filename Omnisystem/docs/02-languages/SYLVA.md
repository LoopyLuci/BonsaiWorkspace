# SYLVA Guide - ML & Data Science

**SYLVA** is Omnisystem's ML and data science language, optimized for numerical computing, data processing, and machine learning.

## Overview

- **Purpose**: ML, data science, numerical computing
- **Features**: DataFrames, neural networks, statistics
- **Integration**: Seamless numpy-like operations
- **Performance**: Optimized for GPU and vectorization

## Core Features

### 1. DataFrames
```sylva
// Create from data
let data = vec![
    vec!["Alice", "25", "Engineer"],
    vec!["Bob", "30", "Designer"],
];
let df = DataFrame::from_rows(data)?;

// Filter
let filtered = df.filter("age > 25")?;

// Select columns
let selected = df.select(vec!["name", "role"])?;

// Aggregate
let avg_age = df.aggregate("age", "mean")?;
```

### 2. Neural Networks
```sylva
// Create model
let mut model = Sequential::new();
model.add(Dense::new(784, 128, "relu"));
model.add(Dense::new(128, 10, "softmax"));

// Train
model.fit(&x_train, &y_train, 32, 10)?;

// Predict
let predictions = model.predict(&x_test)?;
```

### 3. Tensors
```sylva
// Create tensor
let t1 = Tensor::new(vec![2, 3, 4]);

// Operations
let t2 = t1.reshape(vec![6, 4])?;
let t3 = t1 + t2;
let t4 = t1.transpose()?;
```

### 4. Statistical Operations
```sylva
// Mean, std, min, max
let mean = data.mean()?;
let std = data.std()?;
let min = data.min()?;
let max = data.max()?;

// Correlation
let corr = df.correlation("col1", "col2")?;
```

### 5. Data Loading
```sylva
// Load CSV
let df = DataFrame::from_csv("data.csv")?;

// Batch loader
let loader = DataLoader::new(&df, 32);
for batch in loader {
    // Process batch
}
```

## Standard Library Modules

- **dataframe** - DataFrames and operations
- **tensor** - Multi-dimensional arrays
- **nn** - Neural networks
- **optimizer** - Training optimizers
- **loss** - Loss functions
- **activation** - Activation functions
- **preprocessing** - Data preprocessing

## Common Patterns

### Feature Engineering
```sylva
// Normalize
let normalized = df.normalize("column")?;

// One-hot encode
let encoded = df.one_hot_encode("category")?;

// Create derived feature
df.add_column("new_feature", |row| {
    row["col1"] * row["col2"]
})?;
```

### Model Training
```sylva
let mut model = Sequential::new();
model.add(Dense::new(10, 64, "relu"));
model.add(Dense::new(64, 1, "sigmoid"));

model.compile(
    Optimizer::Adam { lr: 0.001 },
    Loss::BinaryCrossentropy
)?;

model.fit(&x_train, &y_train, 32, 100)?;

let eval_loss = model.evaluate(&x_test, &y_test)?;
```

### Inference
```sylva
let predictions = model.predict(&new_data)?;
let classes = predictions.argmax()?;
```

## Best Practices

1. **Data Validation**: Always validate input data
2. **Normalization**: Normalize features before training
3. **Train/Test Split**: Use proper evaluation sets
4. **Monitoring**: Track training metrics
5. **Serialization**: Save trained models

## Related Documentation

- [API Reference](../05-reference/SYLVA_API.md)
- [Building Data Pipelines](../04-guides/DATA_PIPELINES.md)
- [Neural Networks](../13-neural-network/NETWORKS.md)

---

**Status**: Production Ready | **Updated**: 2026-06-16
