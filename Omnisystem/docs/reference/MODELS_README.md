# BonsAI Model Registry

Central registry and management system for all BonsAI models.

## Overview

The BonsAI Model Registry provides:
- ✅ Model discovery and registration
- ✅ Version management
- ✅ Metadata storage
- ✅ Hardware requirements tracking
- ✅ Integration with BUEB allocation

## Components

### bonsai-model-registry
**Location**: `crates/bonsai-model-registry/`

Central registry for all available models.

**Provides**:
- Model registration API
- Version tracking
- Metadata management
- Hardware requirements
- Integration lookups

### bonsai-model-scanner
**Location**: `crates/bonsai-model-scanner/`

Scans and catalogs available models.

**Functionality**:
- Discover models in `models/` directory
- Catalog models in `D:\Models\Custom\`
- Extract metadata
- Build model index

### bonsai-model-converter
**Location**: `crates/bonsai-model-converter/`

Convert models between formats.

**Supported conversions**:
- PyTorch ↔ ONNX
- PyTorch ↔ GGUF
- Safetensors ↔ PyTorch
- Other format conversions

## Model Registry Structure

### Public Models (Repository)

```
Registry Entry:
{
  "name": "model-name",
  "path": "models/base-models/model-name",
  "type": "base|quantized|checkpoint",
  "size": "bytes",
  "hardware_requirements": {
    "min_memory": "bytes",
    "preferred_device": "cpu|gpu",
    "precision": "fp32|fp16|int8|int4"
  }
}
```

### Private Models (External Storage)

```
Registry Entry:
{
  "name": "octopus-ai-model",
  "path": "D:\\Models\\Custom\\octopus-ai-model",
  "type": "private-finetuned",
  "size": "312000000",
  "hardware_requirements": {
    "min_memory": "600000000",
    "preferred_device": "gpu",
    "precision": "auto"
  }
}
```

## Integration with BUEB

The model registry integrates with BUEB for automatic device allocation:

```rust
use bonsai_backend::*;
use bonsai_model_registry::*;

// Initialize
initialize()?;

// Get model from registry
let model = registry.get_model("octopus-ai-model")?;

// Get hardware requirements from model
let requirements = model.hardware_requirements();

// Allocate device
let allocation = allocate(&requirements)?;

// Load model on allocated device
let loaded = model.load_on_device(&allocation)?;
```

## Using the Model Registry

### Discovering Models

```rust
use bonsai_model_registry::*;

let registry = ModelRegistry::new()?;

// List all public models
let public_models = registry.list_public()?;

// List all available models (including private)
let all_models = registry.list_all()?;

// Find model by name
let model = registry.find("octopus-ai-model")?;
```

### Registering New Models

```rust
use bonsai_model_registry::*;

let registry = ModelRegistry::new()?;

// Register a new model
registry.register(ModelMetadata {
    name: "my-model".to_string(),
    path: PathBuf::from("models/base-models/my-model"),
    model_type: ModelType::Base,
    size_bytes: 500_000_000,
    hardware_requirements: HardwareRequirements {
        min_memory_bytes: 500_000_000,
        preferred_device: Device::Auto,
        precision: Precision::Auto,
    },
})?;
```

### Getting Model Information

```rust
let model = registry.get_model("octopus-ai-model")?;

println!("Name: {}", model.name);
println!("Type: {:?}", model.model_type);
println!("Size: {} MB", model.size_bytes / 1_000_000);
println!("Min Memory: {} MB", 
    model.hardware_requirements.min_memory_bytes / 1_000_000);
```

## Model Types

| Type | Location | Public | Safe to Commit |
|------|----------|--------|----------------|
| Base | `models/base-models/` | ✅ | ✅ |
| Quantized | `models/quantized/` | ✅ | ✅ |
| Checkpoint | `models/checkpoints/` | ✅ | ✅ |
| Private Finetuned | `D:\Models\Custom\` | ❌ | ❌ |

## Hardware Requirements Tracking

Each registered model includes:
- Minimum memory requirement
- Preferred device type (CPU/GPU)
- Supported precisions
- Optimal batch size
- Expected inference latency

Example:

```json
{
  "hardware_requirements": {
    "min_memory_bytes": 600000000,
    "preferred_device": "gpu",
    "supported_precisions": ["fp16", "int8", "int4"],
    "optimal_batch_size": 1,
    "estimated_latency_ms": 50
  }
}
```

## Model Metadata

Each model entry contains:
- **Name**: Unique identifier
- **Path**: File system location
- **Type**: base, quantized, checkpoint, private
- **Size**: Total size in bytes
- **Architecture**: Model architecture type
- **Framework**: PyTorch, TensorFlow, ONNX, etc.
- **Version**: Model version
- **License**: License information
- **Hardware Requirements**: As above
- **Training Data**: Source of training data
- **Use Cases**: Intended use cases

## Registry Location

The model registry can be:
1. **File-based**: JSON files in `models/`
2. **Database**: SQLite in `.bonsai/models.db`
3. **In-memory**: Cached in application memory

Current implementation uses file-based registry for simplicity.

## Future Enhancements

- [ ] Model versioning system
- [ ] Automatic model discovery
- [ ] Hardware capability matching
- [ ] Model quality metrics
- [ ] Performance benchmarks
- [ ] Model comparison tools
- [ ] Automatic BUEB optimization

## Integration Points

- **BUEB**: Automatic hardware allocation based on model requirements
- **KDB**: Store model metadata in knowledge database
- **BMF**: Distribute models across network
- **CLI**: Command-line model management tools

---

**Last Updated**: June 3, 2026  
**Status**: ✅ Registry System Ready  
**Model Tracking**: Comprehensive metadata  
**BUEB Integration**: Full support
