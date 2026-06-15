# Model Configurations

Configuration files, metadata, and model-specific settings.

## What Goes Here

- `config.json` files for models
- Model hyperparameters
- Tokenizer configurations
- Generation settings
- Architecture specifications
- Training configurations

## Current Configurations

### Octopus AI v1

**File**: `octopus-v1-config.json`

Octopus AI server management assistant configuration.

## Adding Configurations

To add model configurations:

1. Name files descriptively: `[model-name]-config.json`
2. Include documentation about:
   - Model architecture
   - Hyperparameters
   - Tokenizer settings
   - Expected input/output formats
   - Any special requirements

## Standard Config Fields

```json
{
  "model_type": "gpt2",
  "architectures": ["GPT2LMHeadModel"],
  "vocab_size": 50257,
  "max_position_embeddings": 1024,
  "hidden_size": 768,
  "num_hidden_layers": 12,
  "num_attention_heads": 12,
  "n_embd": 768,
  "n_positions": 1024,
  "n_head": 12,
  "n_layer": 12
}
```

---

**Last Updated**: June 3, 2026
