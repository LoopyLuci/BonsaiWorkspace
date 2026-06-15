# Bonsai Knowledge Database (KDB)

Production-grade modular knowledge database system with hot-swappable modules, HNSW vector search, and seamless inference integration.

## Features

- **Modular Design**: Load/unload knowledge modules at runtime without inference interruption
- **Vector Search**: HNSW (Hierarchical Navigable Small World) indexing for fast approximate nearest neighbor search
- **Hot Module Reloading**: Update modules without pausing inference
- **Multi-Module Retrieval**: Search across all loaded modules simultaneously
- **SQLite Registry**: Persistent module metadata and statistics
- **Zstd Compression**: Optional compression for large value sets
- **Content Hashing**: BLAKE3 hashes for integrity verification

## Quick Start

```rust
use bonsai_kdb::KdbManager;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Create KDB manager
    let manager = KdbManager::new(
        Path::new("./kdb_data"),
        768,  // embedding dimension
        10,   // top-k for search
    )?;

    // Load a knowledge module
    manager.load_module(Path::new("medical.kmod"))?;

    // Search for relevant knowledge
    let query = vec![0.1; 768]; // 768-dimensional query vector
    let results = manager.search(&query, 10)?;

    for (module_name, content, distance) in results {
        println!("Module: {}", module_name);
        println!("Content: {}", content);
        println!("Distance: {}", distance);
    }

    // Reload module for updates
    manager.reload_module("medical", Path::new("medical_updated.kmod"))?;

    // List loaded modules
    for module in manager.list_loaded_modules() {
        println!("Loaded: {} ({})", module.name, module.entry_count);
    }

    Ok(())
}
```

## Architecture

### Three-Layer Design

1. **KdbStore**: Persistent SQLite registry of all known modules
2. **KdbRetriever**: In-memory hot-swappable module loader with vector search
3. **KdbManager**: High-level API combining store and retriever

### Module Format (.kmod)

A .kmod file is a ZIP archive containing:

```
medical.kmod
├── manifest.json          # Module metadata
├── index.hnsw            # HNSW vector index
├── values.txt            # Text chunks (one per line)
└── values.txt.zst        # (Optional) Zstd-compressed chunks
```

**manifest.json**:
```json
{
  "id": "uuid-here",
  "name": "medical",
  "version": "1.0.0",
  "domain": "medicine",
  "description": "Medical knowledge base",
  "dim": 768,
  "entry_count": 5000,
  "distance": "Cosine",
  "created_at": "2025-06-01T00:00:00Z",
  "blake3_index": "hash...",
  "blake3_values": "hash..."
}
```

## API Overview

### KdbManager

Main API for module lifecycle management:

```rust
// Initialize manager
let manager = KdbManager::new(base_dir, dim, top_k)?;

// Load and manage modules
manager.load_module(Path::new("module.kmod"))?;
manager.unload_module("module_name", false)?;  // keep disk
manager.reload_module("module_name", Path::new("updated.kmod"))?;

// Search
let results = manager.search(&query_vector, 10)?;
// Vec<(module_name, content, distance)>

// Module info
let modules = manager.list_loaded_modules();
let is_loaded = manager.is_module_loaded("medical");

// Create modules from datasets
let kmod_path = manager
    .create_module_from_dataset(
        Path::new("dataset.jsonl"),
        "new_module",
        Path::new("./modules"),
        |content| {
            // Your embedding function
            Ok(vec![0.1; 768])
        },
    )
    .await?;
```

### KdbRetriever

In-memory retriever (used by KdbManager):

```rust
let retriever = KdbRetriever::new(768, 10);

// Load module
retriever.load_module("name", Path::new("./modules/name"))?;

// Search
let contexts = retriever.retrieve(&query)?;

// Format as system prompt
let prompt_prefix = retriever.format_context_prefix(&query)?;

// Manage modules
retriever.unload_module("name");
let modules = retriever.list_modules();
```

### KdbStore

Persistent registry (used by KdbManager):

```rust
let store = KdbStore::open(Path::new("./kdb_data"))?;

// Register a module
store.register_module(&manifest, &module_dir)?;

// List known modules
let modules = store.list_modules()?;

// Unregister
store.unregister_module("name")?;

// Get module directory
let dir = store.module_dir("name");
```

## Creating Knowledge Modules

### From JSONL Dataset

```rust
let kmod_path = manager
    .create_module_from_dataset(
        Path::new("documents.jsonl"),
        "documents",
        Path::new("./output"),
        embedding_fn,
    )
    .await?;
```

**Input format** (documents.jsonl):
```jsonl
{"id": "1", "content": "First document...", "embedding": [0.1, 0.2, ...]}
{"id": "2", "content": "Second document...", "embedding": [0.3, 0.4, ...]}
```

Or let the embedding function generate them:
```jsonl
{"id": "1", "content": "First document..."}
{"id": "2", "content": "Second document..."}
```

### Manual Module Creation

```rust
// 1. Create HNSW index
let index = HnswIndex::new(768)?;
for (i, embedding) in embeddings.iter().enumerate() {
    index.add(i, embedding)?;
}
index.save(Path::new("index.hnsw"))?;

// 2. Save chunks
std::fs::write("values.txt", chunks.join("\n"))?;

// 3. Create manifest
let manifest = ModuleManifest {
    id: Uuid::new_v4(),
    name: "my_module".to_string(),
    version: "1.0.0".to_string(),
    // ... other fields
};

// 4. Save as ZIP
let file = std::fs::File::create("module.kmod")?;
let mut zip = zip::ZipWriter::new(file);
zip.start_file("manifest.json", Default::default())?;
zip.write_all(serde_json::to_string(&manifest)?.as_bytes())?;
// ... add other files
zip.finish()?;
```

## Performance Characteristics

### Vector Search

- **Time Complexity**: O(log N) expected for HNSW search
- **Space Complexity**: O(N) for HNSW index
- **Typical Latency**: <1ms for 1M-vector index at top-k=10

### Module Loading

- **Load Time**: O(M) where M = module size on disk
- **Memory Usage**: ~1-2x module size in RAM for index + vectors
- **Hot Reload**: Sub-millisecond switchover (no inference pause)

### Multi-Module Search

- Modules searched in parallel (if using rayon feature)
- Results merged and sorted by distance
- Cost = max(module search times) + sort time

## Distance Metrics

Supported distance functions (configurable in manifest):

- **Cosine**: `-1 to 1`, normalized similarity (default)
- **Euclidean**: `0 to ∞`, Cartesian distance
- **Manhattan**: `0 to ∞`, taxicab distance

## Integration with Inference

### System Prompt Injection

Use retrieved context to augment prompts:

```rust
let context = retriever.format_context_prefix(&query_embedding)?;
let prompt = format!("{}Your query here", context);
// Injected into LLM system prompt
```

### Streaming Search

For real-time retrieval during generation:

```rust
// Pre-compute embeddings for user queries
let user_query_embedding = embedding_model.embed("What is X?")?;
let knowledge = retriever.retrieve(&user_query_embedding)?;

// Use knowledge to constrain generation
for token in model.generate_streaming(&knowledge, prompt) {
    yield token;
}
```

## Error Handling

All operations return `Result<T>` with `KdbError`:

```rust
use bonsai_kdb::{KdbError, Result};

match manager.load_module(path) {
    Ok(_) => println!("Loaded"),
    Err(KdbError::NotFound(p)) => eprintln!("Module not found: {}", p),
    Err(KdbError::DimMismatch { expected, got }) => {
        eprintln!("Dimension mismatch: expected {}, got {}", expected, got)
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Testing

Run tests with:

```bash
cargo test -p bonsai-kdb
```

## Limits and Guarantees

- **Embedding Dimension**: Fixed at module creation time; all queries must match
- **Module Size**: No hard limit; typical modules 10MB-1GB
- **Entry Count**: Up to 2^31 entries per module (SQLite limit)
- **Chunk Size**: Each chunk stored as TEXT; keep under 1MB for best performance
- **Top-K**: No hard limit; typical use 10-100 results
- **Concurrent Modules**: System RAM dependent; typical 10-100 simultaneous modules
- **Hot Reload**: Guaranteed no inference pause during module switch

## Tauri Integration

KDB is integrated into desktop app via commands:

```rust
#[tauri::command]
pub fn kdb_load_module(base_dir: String, request: LoadModuleRequest) -> Result<...>

#[tauri::command]
pub fn kdb_search(base_dir: String, request: SearchRequest) -> Result<...>

#[tauri::command]
pub fn kdb_list_modules(base_dir: String) -> Result<...>
```

## Android Integration

See `android-runtime` for Kotlin wrapper `BonsaiKdbClient`:
- Loads modules from sync'd desktop data
- Simple brute-force search (suitable for small modules)
- Optional JNI extension for HNSW on Android

## Future Enhancements

- Streaming module loading for large indices
- Multi-GPU vector search
- Distributed KDB across nodes
- Automatic module compression
- Query result caching
- Module versioning and rollback

## License

Part of the BonsAI project. See LICENSE in root.
