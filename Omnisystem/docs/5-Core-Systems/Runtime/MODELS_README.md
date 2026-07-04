# Poe AI Models

Poe (Pattern of Expression) AI models and components.

## Overview

Poe is a philosophical AI architecture focused on:
- ✅ Expression patterns
- ✅ Context understanding
- ✅ Nuanced responses
- ✅ Multi-domain knowledge

## Directory Structure

```
poe-ai/
├── MODELS_README.md                (This file)
├── README.md                       (Main documentation)
├── context.md                      (Poe context and philosophy)
├── AC_POE_PERSONALITY.md           (Personality model)
│
├── assets/                         (Model assets)
├── blueprints/                     (Model blueprints)
├── config/                         (Configuration files)
├── dist/                           (Distribution files)
├── kdb-modules/                    (Knowledge database modules)
└── node_modules/                   (Dependencies)
```

## Model Components

### Personality Model

**File**: `AC_POE_PERSONALITY.md`

Defines Poe AI personality characteristics and behavioral patterns.

### Knowledge Database Modules

**Location**: `poe-ai/kdb-modules/`

KDB integration for Poe knowledge storage and retrieval.

### Blueprints

**Location**: `poe-ai/blueprints/`

Model architecture blueprints and design specifications.

## Integration

Poe models integrate with:
- **BUEB**: Hardware-aware device allocation
- **KDB**: Knowledge database for context
- **BMF**: Messaging fabric for communication
- **Octopus AI**: Server management integration

## Using Poe Models

### With BUEB Allocation

```python
from omnisystem_backend import *

initialize()

# Poe models benefit from CPU allocation
task = TaskRequirements(
    task_type=TaskType.Inference,
    estimated_memory_bytes=1_000_000_000,  # 1 GB
    precision=Precision.Auto,
    allow_fallback=True
)

allocation = allocate(task)
print(f"Using: {allocation.devices[0].device_type}")
```

### Accessing Personality

```python
import json

personality_file = "poe-ai/AC_POE_PERSONALITY.md"
with open(personality_file, 'r') as f:
    personality = f.read()
```

## Model Status

- ✅ Personality model defined
- ✅ KDB modules available
- ✅ Blueprint architecture ready
- ⚠️ Full implementation in progress

## Configuration

Configuration files located in `poe-ai/config/`:
- Model settings
- KDB integration settings
- Behavioral parameters
- Integration settings

## Adding Poe Models

To extend Poe models:

1. Document in `blueprints/`
2. Add personality traits to `AC_POE_PERSONALITY.md`
3. Add KDB modules to `kdb-modules/`
4. Update configuration in `config/`
5. Test with BUEB allocation

---

**Last Updated**: June 3, 2026
