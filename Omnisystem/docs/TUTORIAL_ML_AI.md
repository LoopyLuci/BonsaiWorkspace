# Tutorial: Build an ML/AI System with SYLVA

**Complete walkthrough building a real-world machine learning pipeline with neural networks**

---

## Overview

We'll build a complete ML system that:
- Loads and preprocesses image data
- Builds a convolutional neural network (CNN)
- Trains with data augmentation
- Evaluates on test set
- Saves the trained model
- Makes predictions on new data

**Time**: 45-60 minutes  
**Prerequisites**: SYLVA Language Guide, API_SYLVA.md  
**Difficulty**: Intermediate

---

## Step 1: Project Setup

### Create project structure

```bash
mkdir omnisystem-mnist-classifier
cd omnisystem-mnist-classifier
touch main.sy
mkdir data models
```

### Create main.sy

```sylva
// main.sy - MNIST Digit Classifier

use sylva::nn::*
use sylva::optim::*
use sylva::dataset::*
use sylva::loss::*

fun main() -> Result<()> {
    println!("MNIST Classifier with SYLVA")
    println!("Loading data...")
    
    let train_data = load_mnist_data("train")?
    let test_data = load_mnist_data("test")?
    
    println!("Training model...")
    let mut model = create_model()
    train_model(&mut model, &train_data)?
    
    println!("Evaluating model...")
    evaluate_model(&model, &test_data)?
    
    Ok(())
}

fun create_model() -> Sequential {
    Sequential::new()
        .add(Conv2d::new(1, 32, (3, 3)).with_padding(1))
        .add(Dense::new(32 * 28 * 28, 128))
        .add(Dense::new(128, 10))
}
```

### Run It

```bash
omnisystem run main.sy
# MNIST Classifier with SYLVA
# Loading data...
# Training model...
```

---

## Step 2: Data Loading

### Load MNIST data

```sylva
fun load_mnist_data(split: &str) -> Result<DataLoader> {
    // Load MNIST dataset
    let dataset = Dataset::load_mnist(split)?
    
    // Create data loader
    let loader = DataLoader::new(dataset)
        .with_batch_size(32)
        .with_shuffle(true)
        .with_num_workers(4)
    
    Ok(loader)
}
```

### Verify data

```sylva
fun verify_data() -> Result<()> {
    let loader = load_mnist_data("train")?
    
    println!("Dataset loaded")
    for batch in loader.iter().take(1) {
        println!("Batch shape: {:?}", batch.x.shape())
        println!("Labels shape: {:?}", batch.y.shape())
        println!("Batch size: {}", batch.x.shape()[0])
    }
    
    Ok(())
}
```

---

## Step 3: Build CNN Model

### Define architecture

```sylva
fun create_cnn_model() -> Sequential {
    Sequential::new()
        // Conv block 1
        .add(Conv2d::new(1, 32, (3, 3))
            .with_padding(1)
            .with_stride(1))
        
        // Conv block 2
        .add(Conv2d::new(32, 64, (3, 3))
            .with_padding(1)
            .with_stride(2))  // Downsample
        
        // Flatten and dense
        .add(Dense::new(64 * 14 * 14, 128))
        .add(Dense::new(128, 10))
}
```

### Add activation functions

```sylva
fun create_model_with_activations() -> Sequential {
    Sequential::new()
        .add(Conv2d::new(1, 32, (3, 3)).with_padding(1))
        .add(Activation::relu())
        
        .add(Conv2d::new(32, 64, (3, 3)).with_padding(1))
        .add(Activation::relu())
        
        .add(Dense::new(64 * 28 * 28, 128))
        .add(Activation::relu())
        .add(Dropout::new(0.5))
        
        .add(Dense::new(128, 10))
        .add(Activation::softmax())
}
```

---

## Step 4: Training Loop

### Basic training

```sylva
fun train_model(
    model: &mut Sequential,
    train_loader: &DataLoader
) -> Result<()> {
    let mut optimizer = Adam::new(0.001)
    let num_epochs = 10
    
    for epoch in 0..num_epochs {
        let mut total_loss = 0.0
        let mut count = 0
        
        for batch in train_loader {
            // Forward pass
            let predictions = model.forward(&batch.x)?
            
            // Loss calculation
            let loss = cross_entropy(&predictions, &batch.y)
            
            // Backward pass
            model.backward(&loss)?
            
            // Update weights
            optimizer.step(model.parameters())
            optimizer.zero_grad()
            
            total_loss += loss
            count += 1
        }
        
        let avg_loss = total_loss / count as f32
        println!("Epoch {}: loss = {:.4}", epoch, avg_loss)
    }
    
    Ok(())
}
```

### Advanced training with validation

```sylva
fun train_with_validation(
    model: &mut Sequential,
    train_loader: &DataLoader,
    val_loader: &DataLoader,
    num_epochs: i32
) -> Result<()> {
    let mut optimizer = Adam::new(0.001)
    let mut best_val_loss = f32::INFINITY
    let patience = 3
    let mut patience_counter = 0
    
    for epoch in 0..num_epochs {
        // Training phase
        let mut train_loss = 0.0
        let mut count = 0
        
        for batch in train_loader {
            let pred = model.forward(&batch.x)?
            let loss = cross_entropy(&pred, &batch.y)
            
            model.backward(&loss)?
            optimizer.step(model.parameters())
            optimizer.zero_grad()
            
            train_loss += loss
            count += 1
        }
        
        // Validation phase
        let val_loss = evaluate_loss(model, val_loader)?
        
        println!("Epoch {}: train={:.4}, val={:.4}", 
            epoch, train_loss / count as f32, val_loss)
        
        // Early stopping
        if val_loss < best_val_loss {
            best_val_loss = val_loss
            patience_counter = 0
            model.save("best_model.bin")?
        } else {
            patience_counter += 1
            if patience_counter >= patience {
                println!("Early stopping at epoch {}", epoch)
                break
            }
        }
    }
    
    Ok(())
}
```

---

## Step 5: Evaluation

### Compute accuracy

```sylva
fun evaluate_model(
    model: &Sequential,
    test_loader: &DataLoader
) -> Result<()> {
    let mut correct = 0
    let mut total = 0
    
    for batch in test_loader {
        let predictions = model.forward(&batch.x)?
        
        // Get predicted class
        let pred_classes = predictions.argmax()
        
        // Get true classes
        let true_classes = batch.y.argmax()
        
        // Compare
        for (p, t) in pred_classes.iter().zip(true_classes.iter()) {
            if p == t { correct += 1 }
            total += 1
        }
    }
    
    let accuracy = correct as f32 / total as f32
    println!("Test Accuracy: {:.2}%", accuracy * 100.0)
    
    Ok(())
}

fun evaluate_loss(
    model: &Sequential,
    loader: &DataLoader
) -> Result<f32> {
    let mut total_loss = 0.0
    let mut count = 0
    
    for batch in loader {
        let pred = model.forward(&batch.x)?
        let loss = cross_entropy(&pred, &batch.y)
        total_loss += loss
        count += 1
    }
    
    Ok(total_loss / count as f32)
}
```

---

## Step 6: Data Augmentation

### Add augmentation

```sylva
fun create_loader_with_augmentation(
    dataset: &Dataset
) -> DataLoader {
    let augmentation = DataAugmentation::new()
        .add_random_flip(0.5, true)      // 50% flip
        .add_random_rotation(15.0)        // ±15 degrees
        .add_random_crop(28)              // Random crop
        .add_random_noise(0.1)            // Gaussian noise
    
    DataLoader::new(dataset)
        .with_batch_size(32)
        .with_shuffle(true)
        .with_augmentation(augmentation)
}
```

---

## Step 7: Model Persistence

### Save model

```sylva
fun save_trained_model(model: &Sequential) -> Result<()> {
    // Save entire model
    model.save("models/mnist_model.bin")?
    
    // Save state dict (weights only)
    model.save_state_dict("models/mnist_weights.bin")?
    
    // Export to ONNX for deployment
    model.export_onnx("models/mnist_model.onnx")?
    
    println!("Model saved")
    Ok(())
}

fun load_trained_model() -> Result<Sequential> {
    let model = Sequential::load("models/mnist_model.bin")?
    println!("Model loaded")
    Ok(model)
}
```

---

## Step 8: Inference

### Make predictions

```sylva
fun make_predictions(
    model: &Sequential,
    test_images: &Tensor
) -> Result<()> {
    // Forward pass
    let predictions = model.forward(test_images)?
    
    // Get class predictions
    let classes = predictions.argmax()
    
    // Get confidence scores
    let confidences = predictions.max()
    
    for (i, (class, confidence)) in 
        classes.iter().zip(confidences.iter()).enumerate() {
        println!("Image {}: digit={}, confidence={:.2}%", 
            i, class, confidence * 100.0)
    }
    
    Ok(())
}
```

### Batch prediction

```sylva
fun predict_batch(
    model: &Sequential,
    images: &Tensor
) -> Result<Vec<(u32, f32)>> {
    let predictions = model.forward(images)?
    let classes = predictions.argmax()
    let scores = predictions.max()
    
    let results: Vec<_> = classes
        .iter()
        .zip(scores.iter())
        .map(|(c, s)| (*c, *s))
        .collect()
    
    Ok(results)
}
```

---

## Step 9: Complete ML Pipeline

### Full system

```sylva
fun main() -> Result<()> {
    println!("=== MNIST Classification Pipeline ===\n")
    
    // Load data
    println!("1. Loading data...")
    let train_data = load_mnist_data("train")?
    let val_data = load_mnist_data("val")?
    let test_data = load_mnist_data("test")?
    
    // Create model
    println!("2. Creating model...")
    let mut model = create_model_with_activations()
    
    // Train
    println!("3. Training model...")
    train_with_validation(
        &mut model,
        &train_data,
        &val_data,
        10
    )?
    
    // Load best model
    println!("4. Loading best model...")
    model = Sequential::load("best_model.bin")?
    
    // Evaluate
    println!("5. Evaluating on test set...")
    evaluate_model(&model, &test_data)?
    
    // Save
    println!("6. Saving model...")
    save_trained_model(&model)?
    
    println!("\n✓ Pipeline complete!")
    Ok(())
}
```

---

## Step 10: Advanced Topics

### Custom loss function

```sylva
fun custom_loss(pred: &Tensor, target: &Tensor, alpha: f32) -> f32 {
    let ce = cross_entropy(pred, target)
    let focal_weight = (1.0 - pred.max()).pow(2.0)
    ce * focal_weight + alpha * 0.1
}
```

### Learning rate scheduling

```sylva
fun train_with_schedule(
    model: &mut Sequential,
    loader: &DataLoader,
    num_epochs: i32
) -> Result<()> {
    for epoch in 0..num_epochs {
        // Decay learning rate
        let lr = 0.001 * (0.9_f32.powf(epoch as f32))
        let mut optimizer = Adam::new(lr)
        
        // Training...
        println!("Epoch {} (lr={:.5})", epoch, lr)
    }
    
    Ok(())
}
```

### Gradient clipping

```sylva
fun train_with_clipping(
    model: &mut Sequential,
    loader: &DataLoader
) -> Result<()> {
    let max_grad_norm = 1.0
    
    for batch in loader {
        let pred = model.forward(&batch.x)?
        let loss = cross_entropy(&pred, &batch.y)
        
        model.backward(&loss)?
        
        // Clip gradients
        let grads = model.get_gradients()
        let grad_norm = grads.iter()
            .map(|g| g.norm())
            .sum::<f32>()
        
        if grad_norm > max_grad_norm {
            let scale = max_grad_norm / grad_norm
            // Apply scaling...
        }
        
        // Update...
    }
    
    Ok(())
}
```

---

## Testing Checklist

- [ ] Data loads correctly (shape verified)
- [ ] Model creates without errors
- [ ] Single batch trains without errors
- [ ] Loss decreases over epochs
- [ ] Validation loss tracked
- [ ] Model saves to disk
- [ ] Model loads from disk
- [ ] Inference works on test data
- [ ] Accuracy computed correctly
- [ ] Early stopping triggers

---

## Performance Tips

1. **Batch Size**: Use 32-128 for GPUs, 8-32 for CPUs
2. **Learning Rate**: Start with 0.001, decay by 0.9 each epoch
3. **Epochs**: Train until validation loss plateaus (5-15 typically)
4. **Data Augmentation**: Improves generalization significantly
5. **Early Stopping**: Save best model, stop if no improvement

---

## Exercises

### 1. ResNet Architecture
Implement a deeper network with residual connections

### 2. Multi-class Metrics
Add precision, recall, F1-score calculations

### 3. Confusion Matrix
Visualize which digits are confused with each other

### 4. Transfer Learning
Load pretrained ImageNet model, fine-tune on MNIST

### 5. Ensemble Model
Train multiple models, average predictions

---

## Next Steps

- Deploy model using [DEPLOYMENT.md](DEPLOYMENT.md)
- Optimize performance with [PERFORMANCE.md](PERFORMANCE.md)
- Monitor in production with [OPERATIONS.md](OPERATIONS.md)
- Read [API_SYLVA.md](API_SYLVA.md) for advanced features

---

**Congratulations!** You've built a complete ML pipeline. From here, apply to your own datasets and deploy to production.
