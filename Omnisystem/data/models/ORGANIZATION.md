# Public Models Organization

Complete guide to the models directory structure.

## Directory Layout

```
models/
├── README.md                       # Main documentation
├── ORGANIZATION.md                 # This file
│
├── base-models/                    # Complete pre-trained models
│   ├── README.md
│   └── [model directories]
│       ├── pytorch_model.bin
│       ├── config.json
│       ├── tokenizer.json
│       └── README.md
│
├── quantized/                      # GGUF quantized models (efficient)
│   ├── README.md
│   ├── ggml-vocab-aquila.gguf
│   ├── ggml-vocab-baichuan.gguf
│   ├── ggml-vocab-bert-bge.gguf
│   └── [19 more GGUF vocabulary files]
│
├── configs/                        # Model configurations
│   ├── README.md
│   ├── octopus-v1-config.json
│   └── [other config files]
│
└── checkpoints/                    # Training checkpoints (reserved)
    └── [checkpoint directories]
```

## Categories Explained

### base-models/
**Purpose**: Complete, ready-to-use pre-trained models

**Examples**:
- DistilGPT-2 (small language model)
- BERT variants
- Other pre-trained architectures

**Size limit**: < 500 MB preferred

**Use case**: Direct inference, fine-tuning, transfer learning

### quantized/
**Purpose**: Efficient GGUF format models for CPU inference

**Current files**: 19 vocabulary/tokenizer GGUF files

**Size**: KB-MB range (vocabulary files)

**Use case**: Fast CPU inference with llama.cpp

**Advantages**:
- ✅ Smaller than full models
- ✅ Load quickly
- ✅ CPU-friendly
- ✅ Good for embedded systems

### configs/
**Purpose**: Model configurations and metadata

**Current files**:
- `octopus-v1-config.json` - Octopus AI v1 configuration

**Contents**:
- Architecture specifications
- Hyperparameters
- Tokenizer settings
- Generation parameters

### checkpoints/
**Purpose**: Training checkpoints and intermediate states

**Status**: Reserved for future use

**Use case**: Resume training, experiment tracking

---

## Model Statistics

```
Total Quantized Files: 19 GGUF vocabulary files
Total Size: ~35 MB (efficient!)
Public Models: Ready for expansion
Private Models: D:\Models\Custom\ (external storage)
```

## Adding Models

### Adding a Base Model

```bash
# Create directory
mkdir -p models/base-models/my-model

# Add files
cp model.bin models/base-models/my-model/pytorch_model.bin
cp config.json models/base-models/my-model/
cp tokenizer.json models/base-models/my-model/

# Add documentation
echo "Model documentation" > models/base-models/my-model/README.md
```

### Adding a Quantized Model

```bash
# Simply copy the GGUF file
cp model.gguf models/quantized/

# Update quantized/README.md with new model info
```

### Adding Configuration

```bash
# Copy config file
cp model-config.json models/configs/my-model-config.json

# Ensure it's properly documented
```

## Public Repository Safety

All files in this directory are:
- ✅ Safe to commit to git
- ✅ Safe to push to GitHub
- ✅ No sensitive information
- ✅ Properly licensed
- ✅ Documented

## Private Model Storage

**Important**: Private, fine-tuned, or proprietary models go to:
```
D:\Models\Custom\
```

**Not in**: This repository

**Why**: Prevents accidental GitHub uploads of proprietary work

## Integration with BUEB

Models in this directory work seamlessly with BUEB hardware allocation:

```python
from bonsai_backend import *

# Initialize hardware detection
initialize()

# Allocate device for your model
task = TaskRequirements(
    task_type=TaskType.Inference,
    estimated_memory_bytes=500_000_000,  # 500 MB
    precision=Precision.Auto,
    allow_fallback=True
)

allocation = allocate(task)

# Load model with optimal device
from transformers import AutoModel
model = AutoModel.from_pretrained("models/base-models/my-model")
```

## Organization Principles

1. **Clear separation**: Base models, quantized, configs, checkpoints
2. **Documentation**: README in each subdirectory
3. **Size-appropriate**: Keep files repository-friendly
4. **Public-safe**: Never commit proprietary models
5. **Scalable**: Easy to add new models
6. **Accessible**: Well-organized for developers

## Future Expansion

Expected additions:
- [ ] DistilGPT-2 base model
- [ ] BERT base model
- [ ] Additional GGUF models
- [ ] Fine-tuned model checkpoints
- [ ] Multilingual model variants

## References

- **BUEB Integration**: `crates/bonsai-backend/BUEB.md`
- **Private Models**: `LOCAL_REFERENCE_DOCS/PRIVATE_MODELS_REFERENCE.md`
- **Main Models README**: `models/README.md`

---

**Last Updated**: June 3, 2026  
**Status**: ✅ Organized and Ready  
**Public Models**: Properly structured  
**GitHub Ready**: Yes - No sensitive information
