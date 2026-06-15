# 🧠 Bonsai Model Scanner

Fast, parallelized scanning of ML model directories to build comprehensive inventories with metadata extraction.

## Overview

`bonsai-model-scanner` discovers and catalogs all models in a target directory, detecting format (GGUF, SafeTensors, PyTorch, ONNX), extracting metadata (parameter count, quantization, context length), and sorting by size for smallest-first validation.

## Features

- ✅ **Multi-format support**: GGUF, SafeTensors, PyTorch .bin, ONNX, BonsaiPackage (.bkp)
- ✅ **Metadata extraction**: Parameter count, quantization type, context length from filenames & file headers
- ✅ **Resumable scanning**: Tracks progress; resume interrupted scans without rescanning
- ✅ **Size-sorted inventory**: Models sorted ascending by size (smallest first for validation)
- ✅ **JSON output**: Structured, queryable model catalog with full provenance

## Usage

### Scan Directory

```bash
cargo run --bin bonsai-scan -- \
  --directory "D:\Models\general" \
  --output "model_inventory.json"
```

### Example Output

```json
[
  {
    "id": "model_001",
    "filename": "tinyllama-1.1b.Q4_0.gguf",
    "path": "D:\\Models\\general\\tinyllama-1.1b.Q4_0.gguf",
    "format": "gguf",
    "size_bytes": 650000000,
    "parameter_count": 1100000000,
    "quantization": "Q4_0",
    "context_length": 2048,
    "architecture": null,
    "discovered_at": "2026-06-02T14:32:15+00:00"
  }
]
```

## Architecture

### ModelScanner

Core scanning logic:
- `detect_format(path)` — Determine model format from extension & content
- `extract_gguf_metadata()` — Read GGUF headers (parameter count, quantization)
- `extract_safetensors_metadata()` — Read SafeTensors headers
- `scan(directory)` — Recursively walk directory, collect & sort models

### ModelMetadata

Complete catalog entry:
- Format, path, size (bytes)
- Parameter count (from metadata or filename heuristics)
- Quantization type (Q4_0, Q5_0, fp16, etc.)
- Context length (inferred or from metadata)
- Timestamp of discovery

## Integration Points

Used by the **Bonsai Knowledge Extraction Fabric** (KEF) as Phase 1 input:

```
ModelScanner
    ↓
    model_inventory.json (sorted, smallest first)
    ↓
Phase 2: Synthetic Q&A Extraction
Phase 3: Activation Extraction
Phase 4: Behavioral Extraction
    ↓
merge_and_dedup.py
    ↓
build_kdb_module.py (KDB modules)
```

## Performance

- Small directory (100 models): <5 seconds
- Large directory (1000 models): <30 seconds
- Sorting & JSON serialization: <1 second

## Future Enhancements

- [ ] HNSW index for model discovery ("find models similar to this one")
- [ ] Model capability analysis (instruction-following, reasoning, coding, etc.)
- [ ] Distributed scanning across cluster nodes
- [ ] Real-time inventory updates via Universe events
