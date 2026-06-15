# Trained Models - Master Index

Complete reference guide to all BonsAI, Poe, and Octopus trained models and their locations.

## Location Overview

```
WHERE ARE THE MODELS?

TRAINED MODEL FILES (NOT IN REPO - PRIVATE):
├── D:\Models\Custom\octopus-ai-model\           (312 MB - Fine-tuned Octopus AI)
└── D:\Models\Custom\octopus-ai-lora\            (10 MB - LoRA Adapter)

SOURCE CODE (IN REPO):
├── crates/octopus-ai/                           (Training scripts, test suite)
├── crates/poe-core/                             (Poe implementation)
├── crates/poe-bonsai-bridge/                    (Integration)
└── crates/bonsai-model-*/                       (Model system components)

TRAINING DATA (IN REPO):
├── training-data/                               (9,000 training + 1,000 validation)
└── data/octopus-corpus/                         (Domain-specific corpus)

REFERENCES (IN THIS DIRECTORY):
├── octopus-ai/README.md                         (Where to find Octopus AI model)
├── poe-ai/README.md                             (Where to find Poe AI)
└── README.md                                    (This file)
```

## Octopus AI Model

### Trained Model Files
- **Location**: `D:\Models\Custom\octopus-ai-model/`
- **Size**: 312 MB
- **Files**: pytorch_model.bin, config.json, tokenizer.json, etc.
- **Status**: ✅ Ready for inference
- **Access**: See `octopus-ai/README.md`

### LoRA Adapter
- **Location**: `D:\Models\Custom\octopus-ai-lora/`
- **Size**: ~10 MB
- **Purpose**: Original adapter before merging

### Training Code
- **Location**: `crates/octopus-ai/`
- **Scripts**:
  - `train_psychopathy.py` - LoRA fine-tuning
  - `merge_and_convert.py` - Model merging
  - `prepare_data.py` - Data generation
  - `train.py` - Full training pipeline
  - `test_suite.py` - Test suite (2,650+ tests)

### Training Data
- **Location**: `training-data/` and `data/octopus-corpus/`
- **Training**: 9,000 instruction-response pairs
- **Validation**: 1,000 examples
- **Format**: JSONL (Hugging Face compatible)

### Configuration
- **Location**: `models/configs/octopus-v1-config.json`
- **Contents**: Model hyperparameters

## Poe AI

### Source Code
- **Locations**: `crates/poe-*`
  - `crates/poe-core/` - Core implementation
  - `crates/poe-boot/` - Bootstrap
  - `crates/poe-mesh/` - Networking
  - `crates/poe-manifestation/` - Manifestation layer
  - `crates/poe-bonsai-bridge/` - BonsAI integration
  - `crates/poe-bush-sim/` - Simulation

### Personality & Architecture
- **Location**: `poe-ai/`
- **Files**:
  - `AC_POE_PERSONALITY.md` - Personality definition
  - `context.md` - Context and philosophy
  - `blueprints/` - Architecture blueprints
  - `assets/` - Model assets
  - `config/` - Configuration
  - `kdb-modules/` - Knowledge modules

### Status
- ✅ Architecture defined
- ✅ Personality modeled
- ✅ Integration ready
- ⚠️ Full fine-tuned model in development

## BonsAI Model System

### Core Components
- **Registry**: `crates/bonsai-model-registry/`
  - Central model discovery and registration
  - Metadata management
  - Version tracking
  - Hardware requirements

- **Scanner**: `crates/bonsai-model-scanner/`
  - Discovers available models
  - Catalogs models
  - Extracts metadata

- **Converter**: `crates/bonsai-model-converter/`
  - Format conversion (PyTorch ↔ ONNX ↔ GGUF)
  - Model optimization

## Public Models (Repository)

### Quantized Models
- **Location**: `models/quantized/`
- **Files**: 19 GGUF vocabulary files (~35 MB)
- **Status**: ✅ Repository-ready

### Base Models
- **Location**: `models/base-models/`
- **Status**: Ready for addition

### Configurations
- **Location**: `models/configs/`
- **Files**: Model configuration files
- **Example**: `octopus-v1-config.json`

### Checkpoints
- **Location**: `models/checkpoints/`
- **Status**: Reserved for training checkpoints

## Directory Tree

```
Z:\Projects\BonsaiWorkspace/

├── models/
│   ├── README.md
│   ├── ORGANIZATION.md
│   │
│   ├── base-models/
│   │   └── README.md
│   │
│   ├── quantized/
│   │   ├── README.md
│   │   └── [19 GGUF files]
│   │
│   ├── configs/
│   │   ├── README.md
│   │   └── octopus-v1-config.json
│   │
│   ├── checkpoints/
│   │   └── [training checkpoints]
│   │
│   └── trained-models/
│       ├── README.md  (THIS FILE)
│       ├── octopus-ai/
│       │   └── README.md (Reference to D:\Models\Custom\octopus-ai-model)
│       └── poe-ai/
│           └── README.md (Poe AI references)
│
├── training-data/
│   ├── README.md
│   ├── train.jsonl (9,000 examples)
│   ├── validation.jsonl (1,000 examples)
│   └── train.txt
│
├── data/
│   └── octopus-corpus/ (Training corpus data)
│
├── crates/
│   ├── octopus-ai/ (Training scripts and tests)
│   ├── poe-*/ (Poe AI implementations)
│   ├── bonsai-model-registry/ (Model system)
│   ├── bonsai-model-scanner/
│   └── bonsai-model-converter/
│
├── poe-ai/ (Poe AI personality and assets)
│   ├── AC_POE_PERSONALITY.md
│   ├── assets/
│   ├── blueprints/
│   ├── config/
│   ├── kdb-modules/
│   └── src/
│
└── D:\Models\Custom/ (PRIVATE - EXTERNAL STORAGE)
    ├── octopus-ai-model/ (312 MB)
    └── octopus-ai-lora/ (10 MB)
```

## Quick Navigation

### I want to...

**Use the trained Octopus AI model**
→ See `octopus-ai/README.md`
→ Load from `D:\Models\Custom\octopus-ai-model/`

**Train a new model**
→ Use training data in `training-data/`
→ Use script at `crates/octopus-ai/train_psychopathy.py`
→ Allocate hardware with BUEB

**Understand Poe AI**
→ Read `poe-ai/AC_POE_PERSONALITY.md`
→ Explore `crates/poe-core/`

**See model configurations**
→ Check `models/configs/`

**Use quantized models**
→ Load from `models/quantized/`

**Work with model system**
→ Use `crates/bonsai-model-registry/`

## Security & Storage

### Private Models
- ✅ Octopus AI: `D:\Models\Custom\octopus-ai-model/`
- ✅ External storage prevents GitHub leaks
- ✅ Protected by .gitignore
- ✅ Local reference documentation only

### Public Assets
- ✅ Training data in repository
- ✅ Source code in repository
- ✅ Configuration in repository
- ✅ Safe for GitHub publication

## Integration with BUEB

All models integrate seamlessly with BUEB hardware allocation:

```python
from bonsai_backend import *

initialize()
allocation = allocate(TaskRequirements(...))
model = load_model(model_path, allocation)
```

---

**Last Updated**: June 3, 2026  
**Octopus AI**: ✅ Trained and ready  
**Poe AI**: ✅ Architecture ready, training in progress  
**BonsAI System**: ✅ Model registry implemented  
**Public Models**: ✅ Quantized and configs available  
**GitHub Ready**: ✅ Yes - no sensitive data exposed
