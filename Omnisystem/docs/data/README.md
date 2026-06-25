# Training Data

Dataset and training resources for model fine-tuning.

## Octopus AI Training Data

### Location
`data/octopus-corpus/` and `training-data/`

### Files

#### JSONL Format (Hugging Face compatible)

**Training Set**:
- **File**: `training-data/train.jsonl`
- **Size**: 9,000 instruction-response pairs
- **Format**: JSONL (one example per line)
- **Fields**: instruction, response, server-management

**Validation Set**:
- **File**: `training-data/validation.jsonl`
- **Size**: 1,000 instruction-response pairs
- **Format**: JSONL
- **Purpose**: Validation during training

**Combined Format**:
- **File**: `training-data/train.txt`
- **Format**: Plain text
- **Purpose**: llama.cpp compatibility

### Data Corpus

**Location**: `data/octopus-corpus/`

**Files**:
- `server-monitoring.jsonl` - Server monitoring examples
- `programming.jsonl` - Programming/scripting examples
- `dpo-preferences.jsonl` - DPO preference pairs

### Data Characteristics

- **Domain**: Server administration and system management
- **Quality**: Curated and synthetic examples
- **Coverage**:
  - SSH configuration
  - System monitoring
  - Package management
  - Network configuration
  - Performance tuning
  - Troubleshooting

## Using Training Data

### Loading with Hugging Face Datasets

```python
from datasets import load_dataset

# Load training data
dataset = load_dataset('json', data_files='training-data/train.jsonl')

# Load validation data
val_dataset = load_dataset('json', data_files='training-data/validation.jsonl')
```

### Loading Raw JSONL

```python
import json

# Load training examples
with open('training-data/train.jsonl', 'r') as f:
    examples = [json.loads(line) for line in f]

print(f"Loaded {len(examples)} training examples")
```

### Example Data Point

```json
{
  "instruction": "How do I configure SSH on Ubuntu?",
  "response": "To configure SSH on Ubuntu...",
  "domain": "server-management"
}
```

## Data Format Specifications

### JSONL Fields

- **instruction** (str): Question or task description
- **response** (str): Expected answer or completion
- **domain** (str): Domain/category (server-management, programming, etc.)

### Plain Text Format

Concatenated instruction + response pairs, separated by newlines.

## Preparing Custom Training Data

To add more training data:

1. Create a JSONL file with instruction-response pairs
2. Place in `training-data/` directory
3. Update splits for training/validation
4. Use with training scripts:

```python
# In training script
from datasets import load_dataset

dataset = load_dataset('json', data_files='training-data/your-data.jsonl')
```

## Data Generation

The training data was generated using:

**Script**: `crates/octopus-ai/prepare_data.py`

**Generation Method**:
- Synthetic example generation
- Server management domain focus
- 9,000 training + 1,000 validation examples
- Blake3 checksums for integrity

## Data Quality

- ✅ Domain-specific (server management)
- ✅ Diverse examples (SSH, monitoring, scripting, etc.)
- ✅ Well-formatted (JSONL compatible)
- ✅ Train/validation split (9K/1K)
- ✅ Checksummed for integrity

## Statistics

| Metric | Value |
|--------|-------|
| Total examples | 10,000 |
| Training examples | 9,000 |
| Validation examples | 1,000 |
| Avg response length | ~150 tokens |
| Domain | Server Management |
| Format | JSONL + Plain Text |

## Integration with BUEB

When training with BUEB allocation:

```python
from bonsai_backend import *
from datasets import load_dataset

# Initialize hardware
initialize()

# Allocate for training
task = TaskRequirements(
    task_type=TaskType.Training,
    estimated_memory_bytes=8_000_000_000,
    precision=Precision.Auto,
    allow_fallback=True
)

allocation = allocate(task)

# Load data
dataset = load_dataset('json', data_files='training-data/train.jsonl')

# Train with allocated device
# ... training code ...
```

## Source Code

Training scripts:
- `crates/octopus-ai/prepare_data.py` - Data preparation
- `crates/octopus-ai/train.py` - Main training script
- `crates/octopus-ai/train_psychopathy.py` - LoRA fine-tuning

## License

Training data is provided for research and development purposes.

---

**Last Updated**: June 3, 2026  
**Total Examples**: 10,000  
**Domain**: Server Management  
**Status**: ✅ Ready for Training
