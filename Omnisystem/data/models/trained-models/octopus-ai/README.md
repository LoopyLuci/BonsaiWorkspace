# Octopus AI Model

Fine-tuned Octopus AI server management assistant model.

## Model Location

**Actual Model Files**: `D:\Models\Custom\octopus-ai-model/`

This is a **PRIVATE MODEL** stored in external storage to prevent accidental GitHub uploads.

## Model Details

### Trained Model
- **Path**: `D:\Models\Custom\octopus-ai-model\`
- **Size**: 312 MB
- **Type**: Fine-tuned causal language model
- **Base**: DistilGPT-2 (82M parameters)
- **Fine-tuning**: LoRA (Rank-4, Alpha-8)
- **Status**: ✅ Ready for inference

### LoRA Adapter
- **Path**: `D:\Models\Custom\octopus-ai-lora\`
- **Size**: ~10 MB
- **Type**: LoRA adapter (before merging)
- **Status**: ✅ Available for further training

## Training Data

- **Location**: `data/octopus-corpus/`
- **Size**: 9,000 instruction-response pairs
- **Files**:
  - `train.jsonl` - 9,000 training examples
  - `validation.jsonl` - 1,000 validation examples
  - `train.txt` - Combined text format

## Model Files

The trained model contains:
- `pytorch_model.bin` - 312.49 MB merged weights
- `config.json` - Model configuration
- `generation_config.json` - Generation settings
- `tokenizer.json` - BPE tokenizer
- `tokenizer_config.json` - Tokenizer configuration
- `vocab.json` - Vocabulary
- `merges.txt` - Merge operations
- `special_tokens_map.json` - Special token mappings

## Loading the Model

```python
from transformers import AutoModelForCausalLM, AutoTokenizer

model_path = "D:\\Models\\Custom\\octopus-ai-model"

model = AutoModelForCausalLM.from_pretrained(model_path)
tokenizer = AutoTokenizer.from_pretrained(model_path)

# Inference
inputs = tokenizer("How do I configure SSH?", return_tensors="pt")
outputs = model.generate(**inputs, max_length=100)
response = tokenizer.decode(outputs[0])
```

## With BUEB Hardware Allocation

```python
from bonsai_backend import *

# Initialize hardware detection
initialize()

# Allocate device for inference
task = TaskRequirements(
    task_type=TaskType.Inference,
    estimated_memory_bytes=600_000_000,  # 600 MB
    precision=Precision.Auto,
    allow_fallback=True
)

allocation = allocate(task)

# Load model on allocated device
model = AutoModelForCausalLM.from_pretrained(model_path)
```

## Performance Metrics

| Hardware | Latency | Throughput | Batch | Precision |
|----------|---------|-----------|-------|-----------|
| CPU (24 cores) | 200-500ms | 2-5 q/s | 1 | INT8 |
| GPU (RTX 3080) | 20-50ms | 20-50 q/s | 4-8 | FP16 |
| Multi-GPU (2×) | 10-20ms | 50-100+ q/s | 16+ | FP16 |

## Training Information

- **Framework**: PyTorch
- **Method**: LoRA fine-tuning
- **Training Steps**: 158
- **Loss Progression**: 2.386 → 1.020 → 1.512
- **Epoch**: 1
- **Training Time**: ~12.5 minutes
- **Hardware**: CPU (Ryzen 9 5900X)

## Access & Security

- ✅ **Private Model**: Stored externally on D: drive
- ✅ **NOT in Repository**: Never committed to git
- ✅ **NOT on GitHub**: Prevented by .gitignore
- ✅ **Local Reference**: Documentation in LOCAL_REFERENCE_DOCS/

## Training Source Code

Fine-tuning scripts are available in:
- `crates/octopus-ai/train_psychopathy.py` - Training script
- `crates/octopus-ai/merge_and_convert.py` - Merge and conversion script
- `crates/octopus-ai/prepare_data.py` - Data preparation script

---

**Last Updated**: June 3, 2026  
**Model Status**: ✅ Ready for Production  
**Storage**: D:\Models\Custom\ (External, Secure)
