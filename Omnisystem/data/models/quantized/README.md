# Quantized Models

GGUF format quantized models for efficient inference on CPU and GPU.

## What is GGUF?

GGUF (GPT-Generated Unified Format) is an efficient model format that:
- ✅ Supports quantization (Q4, Q5, Q8, etc.)
- ✅ Loads quickly on CPU
- ✅ Reduces memory usage
- ✅ Maintains good accuracy
- ✅ Compatible with llama.cpp and other tools

## Available Models

Models in this directory are vocabulary/tokenizer files in GGUF format:

| Model | Size | Purpose |
|-------|------|---------|
| ggml-vocab-aquila.gguf | 4.7 MB | Aquila model vocabulary |
| ggml-vocab-baichuan.gguf | 1.3 MB | Baichuan model vocabulary |
| ggml-vocab-bert-bge.gguf | 613 KB | BGE embedding vocabulary |
| ggml-vocab-command-r.gguf | 11 MB | Command-R vocabulary |
| [Others] | Various | Additional model vocabularies |

## Using GGUF Models

### With llama.cpp

```bash
./main -m models/quantized/model.gguf -p "Your prompt here"
```

### With Python

```python
from ctransformers import AutoModelForCausalLM

model = AutoModelForCausalLM.from_pretrained(
    "models/quantized/model.gguf",
    model_type="llama"
)
```

### With BUEB Hardware Allocation

```python
from bonsai_backend import *

initialize()

# GGUF models are excellent for CPU inference
task = TaskRequirements(
    task_type=TaskType.Inference,
    estimated_memory_bytes=2_000_000_000,  # 2GB for quantized
    precision=Precision.INT8,  # Quantized = INT8/INT4
    allow_fallback=True
)

allocation = allocate(task)
print(f"Using: {allocation.devices[0].device_type}")
print(f"Precision: {allocation.precision}")
```

## Quantization Levels

| Level | Size | Speed | Quality |
|-------|------|-------|---------|
| Q4 | ~25% | ⚡⚡⚡ | Good |
| Q5 | ~35% | ⚡⚡ | Better |
| Q8 | ~50% | ⚡ | Best |
| FP16 | 100% | - | Original |

## Adding New Quantized Models

To add a quantized GGUF model:

1. Place `.gguf` file in this directory
2. Create a model-specific README with:
   - Model name and source
   - Quantization level
   - Vocabulary used
   - Recommended settings
   - License information

## Storage Notes

- Quantized models are perfect for repository storage
- Size-efficient (KB-MB range for vocabularies)
- No need for external storage
- Safe to commit to GitHub

---

**Last Updated**: June 3, 2026
