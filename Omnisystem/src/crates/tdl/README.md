# Bonsai Training Data Library (TDL)

Production-grade SQLite-backed training dataset management system with versioning, quality scoring, and multi-format export.

## Features

- **Versioned Datasets**: Create and manage multiple versions of training data with full history tracking
- **Quality Scoring**: Rate examples from 0.0 to 1.0 for filtering high-quality training data
- **Structured Metadata**: JSON-based metadata storage with source, author, domain, tags, and language fields
- **Multi-Format Export**: Export to JSONL or Apache Parquet formats
- **Transactional Safety**: ACID-compliant SQLite with connection pooling
- **Zero-Copy Semantics**: Efficient handling of large training examples
- **Deduplication**: Content hash tracking to prevent duplicate examples

## Quick Start

```rust
use bonsai_tdl::{TrainingDataLibrary, Metadata, ExportFormat};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize library
    let library = TrainingDataLibrary::new(Path::new("./data/tdl.db")).await?;

    // Create a version
    let version_id = library
        .create_version("1.0.0", "alice", "Initial training data", vec!["nlp".to_string()])
        .await?;

    // Add training examples
    let metadata = Metadata::new()
        .with_source("https://example.com/dataset")
        .with_author("Alice")
        .with_domain("nlp")
        .with_language("en")
        .with_tag("sentiment-analysis");

    let example_id = library
        .add_example(
            version_id,
            "This movie is amazing! I loved it.".to_string(),
            metadata,
            0.95, // Quality score
        )
        .await?;

    // Search by quality
    let high_quality = library.search_by_quality(0.8, 100).await?;
    println!("High-quality examples: {}", high_quality.len());

    // Export dataset
    let export_path = library
        .export_dataset(version_id, ExportFormat::Jsonl, Path::new("export.jsonl"))
        .await?;
    println!("Exported to: {:?}", export_path);

    Ok(())
}
```

## API Overview

### TrainingDataLibrary

Main API for TDL operations:

```rust
// Create or open a library
let library = TrainingDataLibrary::new(db_path).await?;

// Create versions
let version_id = library
    .create_version(version_string, created_by, description, tags)
    .await?;

// Add examples to a version
let example_id = library
    .add_example(version_id, content, metadata, quality_score)
    .await?;

// Search operations
let examples = library.get_examples(tags, limit).await?;
let high_quality = library.search_by_quality(0.8, 100).await?;

// Export datasets
library
    .export_dataset(version_id, ExportFormat::Jsonl, output_path)
    .await?;

// Manage versions
let history = library.get_version_history().await?;
let version = library.get_version(version_id).await?;
let merged_id = library.merge_versions(v1, v2, creator).await?;
```

### Metadata

Structured metadata with builder pattern:

```rust
let metadata = Metadata::new()
    .with_source("https://arxiv.org/abs/2103.15808")
    .with_author("Alice")
    .with_domain("nlp")
    .with_language("en")
    .with_tag("transformers")
    .with_tag("attention");
```

### Export Formats

Supported export formats:

- **JSONL**: One JSON object per line, streaming-friendly
- **Parquet**: Columnar format for data analysis and ML pipelines

```rust
library
    .export_dataset(
        version_id,
        ExportFormat::Jsonl,
        Path::new("data.jsonl"),
    )
    .await?;

library
    .export_dataset(
        version_id,
        ExportFormat::Parquet,
        Path::new("data.parquet"),
    )
    .await?;
```

## Database Schema

See `schema.sql` for the complete SQLite schema. Key tables:

- **versions**: Version metadata and statistics
- **examples**: Individual training examples with content and metadata
- **version_examples**: Junction table for multi-version example tracking
- **datasets**: Exported datasets with checksums

## Quality Score Guidelines

Quality scores should be assigned on a 0.0-1.0 scale:

- **0.9-1.0**: Excellent, publication-ready data
- **0.7-0.9**: Good, suitable for training
- **0.5-0.7**: Acceptable, may need cleaning
- **0.0-0.5**: Poor, use with caution or filter out

## Performance Considerations

- **Indexing**: Queries use indexes on version_id, quality_score, and content_hash
- **Pagination**: Use limit/offset for large datasets
- **Transactions**: All multi-step operations use ACID transactions
- **Connection Pool**: Default pool size is 5 connections
- **Compression**: Content is stored uncompressed by default for fast access

## Thread Safety

- `TrainingDataLibrary` uses `sqlx` with connection pooling (thread-safe)
- Safe to use from async contexts with `.await`
- Connection pool automatically manages concurrent access

## Error Handling

All operations return `Result<T>` with `TdlError` enum:

```rust
use bonsai_tdl::TdlError;

match library.add_example(...).await {
    Ok(id) => println!("Added: {}", id),
    Err(TdlError::InvalidQualityScore(s)) => eprintln!("Score out of range: {}", s),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Testing

Run tests with:

```bash
cargo test -p bonsai-tdl
```

Tests cover:
- Version creation and retrieval
- Example addition and validation
- Quality score filtering
- Version merging
- JSONL and Parquet export
- Metadata building

## Integration

### Tauri Desktop Commands

TDL is integrated into the Bonsai desktop app via Tauri commands:

```rust
#[tauri::command]
pub async fn tdl_create_version(db_path: String, ...) -> Result<...>

#[tauri::command]
pub async fn tdl_add_example(db_path: String, ...) -> Result<...>

#[tauri::command]
pub async fn tdl_search_by_quality(db_path: String, ...) -> Result<...>

#[tauri::command]
pub async fn tdl_export_dataset(db_path: String, ...) -> Result<...>
```

### Android Integration

See `android-runtime` for Kotlin wrappers:
- `BonsaiTdlClient`: Manages local TDL database copy
- Syncs from desktop via background service
- Provides offline access to training examples

## Limits and Guarantees

- **Example Size**: No hard limit; content stored as TEXT (SQLite max ~2GB per DB)
- **Metadata Size**: Metadata stored as JSON; keep under 1MB per example
- **Quality Score**: Must be in range [0.0, 1.0]
- **Version String**: Must be unique; recommend semantic versioning (e.g., "1.0.0")
- **Tags**: Unlimited; stored as JSON array in metadata
- **Concurrent Connections**: Pool size 5; increase via `SqlitePoolOptions`

## License

Part of the BonsAI project. See LICENSE in root.
