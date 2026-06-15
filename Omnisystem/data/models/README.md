# BonsAI Models

Unified model system integrating Octopus AI, Poe AI, and the BonsAI Foundation model registry.

## Quick Start

### List All Available Models

```bash
python models/model_selector.py summary
```

### Get Model Info

```bash
python models/model_selector.py info --model octopus-ai
```

### Find Model Path

```bash
python models/model_selector.py path --model octopus-ai
```

### Filter by Type

```bash
python models/model_selector.py list --type octopus
python models/model_selector.py list --type poe
python models/model_selector.py list --type bonsai
```

## Available Models

### Octopus AI
**Server Management Assistant**

- **Base**: DistilGPT-2 (82M parameters)
- **Fine-tuning**: LoRA (rank-4, alpha-8)
- **Size**: 312 MB
- **Status**: Production-ready
- **Capabilities**: Server administration, system Q&A, configuration guidance, troubleshooting
- **Location**: See [models/trained-models/octopus-ai/README.md](trained-models/octopus-ai/README.md)

### Poe AI
**Pattern of Expression Model**

- **Type**: Personality-driven expression architecture
- **Framework**: Hybrid Rust implementation
- **Components**: Core, Boot, Mesh, Manifestation, BonsAI Bridge, Simulation
- **Status**: Architecture-ready, training in development
- **Capabilities**: Nuanced expression, context understanding, multi-domain knowledge
- **Location**: See [models/trained-models/poe-ai/README.md](trained-models/poe-ai/README.md)

### BonsAI Foundation
**Model Registry & Inference System**

- **Components**: Registry, Scanner, Converter, Inference, Telemetry
- **Framework**: Rust
- **Status**: Production-ready
- **Capabilities**: Model discovery, registration, format conversion, hardware-aware allocation
- **Supported Formats**: PyTorch, ONNX, GGUF, TensorFlow Lite, TensorRT

## Model Directory Structure

```
models/
├── README.md (this file)
├── ORGANIZATION.md
├── MODEL_MANIFEST.json
├── model_selector.py
│
├── base-models/
│   └── README.md - Pre-trained base models
│
├── quantized/
│   ├── README.md
│   └── [19 GGUF vocabulary files - 35 MB total]
│
├── configs/
│   ├── README.md
│   └── octopus-v1-config.json
│
├── checkpoints/
│   └── [Training checkpoints and intermediate models]
│
└── trained-models/
    ├── README.md - Master index of all trained models
    ├── octopus-ai/README.md
    └── poe-ai/README.md
```

## Integration Points

All models integrate seamlessly with:

- **BUEB**: Hardware-aware device allocation (CPU, GPU, multi-GPU)
- **KDB**: Knowledge database integration
- **BMF**: BonsAI messaging framework
- **Model Selector**: Unified discovery and loading interface

## Using Models

### Python Integration

```python
from models.model_selector import ModelSelector

selector = ModelSelector("models/")
models = selector.list_all_models()

octopus = selector.get_model("octopus-ai")
print(f"Found: {octopus.name}")
print(f"Capabilities: {octopus.capabilities}")
```

### Load Model with BUEB

```python
from bonsai_backend import initialize, allocate, TaskType, TaskRequirements
from models.model_selector import ModelSelector

initialize()
requirements = TaskRequirements(task_type=TaskType.Inference)
allocation = allocate(requirements)

selector = ModelSelector("models/")
model_path = selector.get_model_path("octopus-ai")
# Load model using allocation device
```

### Command Line

```bash
# List all models with detailed info
python models/model_selector.py summary

# Get specific model config
python models/model_selector.py info --model octopus-ai --json

# Get model location
python models/model_selector.py path --model poe-ai
```

## Model Configuration Files

Metadata files are stored in each crate's directory:

- `crates/octopus-ai/model.json` - Octopus AI specifications
- `crates/poe-ai/model.json` - Poe AI specifications (if available)
- `crates/bonsai-model-registry/model.json` - BonsAI Foundation system

Central manifest: `models/MODEL_MANIFEST.json`

## Training Data

Located in the repository:

- `training-data/train.jsonl` - 9,000 training examples
- `training-data/validation.jsonl` - 1,000 validation examples
- `data/octopus-corpus/` - Domain-specific training corpus

Format: JSONL (Hugging Face compatible)

## Quantized Models

19 pre-quantized GGUF vocabulary files available in `models/quantized/`:

- CPU-optimized inference
- ~35 MB total size
- Ready for immediate use
- Support for llama.cpp and GGML-based inference

## Model Manifest

The `MODEL_MANIFEST.json` contains:

- Model definitions (name, version, location, capabilities)
- Framework and format information
- Hardware requirements and performance metrics
- Integration status
- Security metadata (public vs. private models)

To regenerate or update: Edit `MODEL_MANIFEST.json` directly or use the model registry components.

## Security & Privacy

- **Public Models**: Stored in repository (`models/`)
- **Private Models**: External storage (`D:\Models\Custom\`)
- **GitHub Safe**: No sensitive locations or private paths in tracked files
- **Protected**: `.gitignore` prevents accidental uploads

For more details, see [models/trained-models/README.md](trained-models/README.md#security--storage)

## Performance Benchmarks

### Octopus AI (DistilGPT-2 + LoRA)

**CPU (Ryzen 9 5900X, 24 cores)**
- Latency: 200-500ms per request
- Throughput: 2-5 queries/second

**GPU (Single)**
- Latency: 20-50ms per request
- Throughput: 20-50 queries/second

**Multi-GPU**
- Throughput: 50-100+ queries/second

## Next Steps

1. **Explore Models**: Run `python models/model_selector.py summary`
2. **Load a Model**: Use the Python API or command line
3. **Use with BUEB**: Allocate hardware and load models dynamically
4. **Extend System**: Add new models via `bonsai-model-registry`

## Support

For issues or questions:
- Check [models/trained-models/README.md](trained-models/README.md)
- Review individual model documentation in respective crates
- Consult `MODEL_MANIFEST.json` for complete metadata

---

**Last Updated**: June 3, 2026  
**Model System Version**: 1.0  
**Status**: Production-ready
