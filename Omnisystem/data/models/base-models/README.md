# Base Models

Pre-trained foundation models and base architectures.

## What Goes Here

Base models are complete, pre-trained models that can be:
- Used directly for inference
- Fine-tuned for specific tasks
- Used as starting points for transfer learning

## Model Categories

### Language Models
- GPT variants (GPT-2, GPT-3, etc.)
- LLaMA models
- Mistral models
- Phi models
- Other transformer-based language models

### Embedding Models
- BERT variants
- RoBERTa
- Sentence-BERT
- Other embedding models

### Specialized Models
- Vision transformers
- Audio models
- Multimodal models

## Size Requirements

- Target: < 500 MB per model
- Larger models should be quantized (move to `../quantized/`)
- Consider GGUF format for large models

## Adding Base Models

To add a base model:

1. Create a subdirectory with the model name
2. Include:
   - Model weights (`.pt`, `.bin`, `.safetensors`)
   - `config.json` - Model configuration
   - `README.md` - Model documentation
   - License file if applicable
   - `tokenizer.json` or `tokenizer.model` if applicable

## Example Structure

```
base-models/
└── distilgpt2/
    ├── pytorch_model.bin
    ├── config.json
    ├── tokenizer.json
    ├── README.md
    └── LICENSE
```

## Public vs Private

- **Public models**: Store in this directory (in repository)
- **Private/fine-tuned models**: Store in `D:\Models\Custom\` (external)

---

**Last Updated**: June 3, 2026
