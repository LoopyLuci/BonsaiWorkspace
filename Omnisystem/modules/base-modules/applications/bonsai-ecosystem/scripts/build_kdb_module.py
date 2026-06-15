#!/usr/bin/env python3
"""
KDB Module Builder

Packages deduplicated knowledge chunks into .kmod files (ZIP archives)
with HNSW vector index, metadata, and full provenance tracking.
"""

import argparse
import json
import zipfile
from pathlib import Path
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, asdict
from datetime import datetime
import struct

@dataclass
class ModuleMetadata:
    """Metadata for a KDB module."""
    name: str
    version: str
    source_model: str
    model_size_bytes: int
    num_chunks: int
    mean_quality_score: float
    extraction_date: str
    domains: List[str]
    extraction_methods: List[str]
    total_tokens: int

class SimpleHNSWIndex:
    """Simplified HNSW index (mock for now, full impl. uses hnswlib)."""

    def __init__(self, dimension: int = 384):
        self.dimension = dimension
        self.entries: List[tuple] = []

    def add_vector(self, idx: int, vector: List[float], metadata: Dict[str, Any]) -> None:
        """Add a vector to the index."""
        self.entries.append((idx, vector, metadata))

    def save(self, path: Path) -> None:
        """Save index to binary file (mock implementation)."""
        # In production, use hnswlib.Index.save()
        with open(path, "wb") as f:
            # Write header
            f.write(b"HNSW")  # Magic
            f.write(struct.pack("<I", len(self.entries)))  # Number of entries
            f.write(struct.pack("<I", self.dimension))  # Dimension

            # Write placeholder (real impl. would write graph structure)
            f.write(b"PLACEHOLDER_INDEX_DATA" * 100)

class KDBModuleBuilder:
    """Build KDB modules from deduplicated chunks."""

    def __init__(self, embedding_dim: int = 384):
        self.embedding_dim = embedding_dim

    def chunk_to_vector(self, chunk: Dict[str, Any]) -> List[float]:
        """Convert a chunk to an embedding vector (mock)."""
        # In production, use actual embedding model
        import hashlib
        import random

        # Mock: hash-based reproducible "embedding"
        content = str(chunk).encode()
        h = hashlib.md5(content).hexdigest()
        random.seed(int(h[:8], 16))
        vector = [random.random() for _ in range(self.embedding_dim)]
        return vector

    def build_module(
        self,
        chunks: List[Dict[str, Any]],
        output_path: Path,
        source_model: str,
        model_size_bytes: int = 0,
    ) -> None:
        """Build a .kmod file from chunks."""
        print(f"   Building KDB module: {output_path.name}")
        print(f"     Chunks: {len(chunks)}")

        # Extract metadata
        domains = set()
        methods = set()
        total_quality = 0.0

        for chunk in chunks:
            if "domain" in chunk:
                domains.add(chunk["domain"])
            if "extraction_method" in chunk:
                methods.add(chunk["extraction_method"])
            total_quality += chunk.get("quality_score", 0.8)

        mean_quality = total_quality / len(chunks) if chunks else 0.0

        # Estimate token count (rough heuristic)
        total_tokens = 0
        for chunk in chunks:
            content = (
                chunk.get("answer")
                or chunk.get("response")
                or chunk.get("concept_description")
                or ""
            )
            total_tokens += len(content.split())  # Rough tokenization

        # Build metadata
        metadata = ModuleMetadata(
            name=f"{source_model}-knowledge",
            version="1.0.0",
            source_model=source_model,
            model_size_bytes=model_size_bytes,
            num_chunks=len(chunks),
            mean_quality_score=mean_quality,
            extraction_date=datetime.now().isoformat(),
            domains=sorted(list(domains)),
            extraction_methods=sorted(list(methods)),
            total_tokens=total_tokens,
        )

        # Create HNSW index
        index = SimpleHNSWIndex(dimension=self.embedding_dim)
        indexed_chunks = []

        for i, chunk in enumerate(chunks):
            vector = self.chunk_to_vector(chunk)
            index.add_vector(i, vector, {"id": chunk.get("id", f"chunk_{i}")})
            indexed_chunks.append(chunk)

        # Create .kmod ZIP archive
        with zipfile.ZipFile(output_path, "w", zipfile.ZIP_DEFLATED) as kmod:
            # Write metadata
            kmod.writestr(
                "metadata.json",
                json.dumps(asdict(metadata), indent=2),
            )

            # Write chunks as JSONL
            chunks_jsonl = "\n".join(json.dumps(c) for c in indexed_chunks)
            kmod.writestr("chunks.jsonl", chunks_jsonl)

            # Write index placeholder
            # In production, write binary HNSW index
            index_placeholder = {
                "type": "hnsw",
                "dimension": self.embedding_dim,
                "num_vectors": len(indexed_chunks),
                "note": "Use hnswlib.Index.load() in production",
            }
            kmod.writestr("index_meta.json", json.dumps(index_placeholder, indent=2))

            # Write README
            readme = f"""# {metadata.name}

## Overview
KDB module containing knowledge extracted from **{metadata.source_model}**.

## Statistics
- **Chunks**: {metadata.num_chunks:,}
- **Total tokens**: {metadata.total_tokens:,}
- **Mean quality score**: {metadata.mean_quality_score:.3f}
- **Domains**: {', '.join(metadata.domains)}
- **Extraction methods**: {', '.join(metadata.extraction_methods)}

## Contents
- `metadata.json` — Module metadata and statistics
- `chunks.jsonl` — Knowledge chunks (one per line)
- `index_meta.json` — Vector index metadata

## Usage

```python
import json
import zipfile

with zipfile.ZipFile("{output_path.name}") as kmod:
    # Load metadata
    metadata = json.loads(kmod.read("metadata.json"))
    print(f"Loaded: {metadata['name']} with {metadata['num_chunks']} chunks")

    # Load chunks
    chunks = [json.loads(line) for line in kmod.read("chunks.jsonl").decode().split("\\n")]
    for chunk in chunks[:3]:
        print(f"  - {chunk.get('id')}: {chunk.get('question', chunk.get('prompt', ''))}")
```

## Integration with Bonsai KDB

```bash
# Register with KDB
bonsai kdb register {output_path.name}

# Search knowledge
bonsai kdb search --module {source_model} "your question"

# Load at inference time
bonsai model infer --with-kdb {source_model} "your prompt"
```

---
Generated: {metadata.extraction_date}
"""
            kmod.writestr("README.md", readme)

        print(f"     ✅ Module created: {output_path}")
        print(f"        Size: {output_path.stat().st_size / 1024 / 1024:.2f} MB")
        print(f"        Quality: {mean_quality:.3f} / 1.0")
        print(f"        Domains: {', '.join(metadata.domains)}")

def main():
    parser = argparse.ArgumentParser(
        description="Build KDB module from extracted knowledge chunks"
    )
    parser.add_argument("--input", required=True, help="Input JSONL file")
    parser.add_argument("--output", required=True, help="Output .kmod file")
    parser.add_argument("--model-name", required=True, help="Source model identifier")
    parser.add_argument("--model-size", type=int, default=0, help="Source model size in bytes")
    parser.add_argument("--embedding-dim", type=int, default=384, help="Embedding vector dimension")
    args = parser.parse_args()

    # Read chunks
    input_path = Path(args.input)
    if not input_path.exists():
        print(f"❌ Input file not found: {args.input}")
        return 1

    print(f"📦 Building KDB module from: {args.input}")

    chunks = []
    with open(input_path, "r", encoding="utf-8") as f:
        for line in f:
            chunks.append(json.loads(line))

    if not chunks:
        print(f"❌ No chunks found in input file")
        return 1

    # Build module
    builder = KDBModuleBuilder(embedding_dim=args.embedding_dim)
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    builder.build_module(chunks, output_path, args.model_name, args.model_size)

    print(f"\n✅ KDB module created: {output_path}")
    print(f"   Chunks: {len(chunks)}")
    print(f"   Output: {output_path.stat().st_size / 1024 / 1024:.2f} MB")

    return 0

if __name__ == "__main__":
    exit(main())
