# Bonsai KEF Project Structure

Complete overview of the Knowledge Extraction Fabric implementation.

## Directory Layout

```
crates/bonsai-kef/
├── Cargo.toml                  # Package manifest with dependencies
├── README.md                   # User guide and feature overview
├── API.md                      # Complete API reference
├── INTEGRATION.md              # Integration patterns with BonsAI ecosystem
├── STRUCTURE.md                # This file
├── src/
│   ├── lib.rs                 # Library root, module definitions
│   ├── error.rs               # Error types and Result type (60 lines)
│   ├── types.rs               # Core data types (280 lines)
│   │   ├── ExtractionMethod enum
│   │   ├── QualityScores struct
│   │   ├── CuratedChunk struct
│   │   ├── KmodPackage struct
│   │   └── ExtractionReport struct
│   ├── model_scanner.rs       # Model detection & introspection (320 lines)
│   │   ├── ModelType enum (Llm, Embedding, Vision, Moe, Other)
│   │   ├── ModelReport struct
│   │   └── ModelScanner (GGUF, safetensors, PyTorch, ONNX detection)
│   ├── synthetic_generator.rs # Synthetic data generation (200 lines)
│   │   ├── SyntheticGeneratorConfig struct
│   │   └── SyntheticDataGenerator
│   │       ├── generate_from_topics()
│   │       ├── generate_from_vocabulary()
│   │       └── generate_beam_search()
│   ├── activation_extractor.rs # Activation clustering (370 lines)
│   │   ├── ActivationExtractorConfig struct
│   │   ├── ActivationSample struct
│   │   ├── ActivationCluster struct (with centroid & K-means)
│   │   └── ActivationExtractor
│   │       ├── add_sample()
│   │       └── cluster() → K-means algorithm
│   ├── attention_extractor.rs # Attention → triplets (180 lines)
│   │   ├── AttentionExtractorConfig struct
│   │   ├── KnowledgeTriplet struct
│   │   └── AttentionExtractor
│   │       ├── extract_from_attention()
│   │       └── validate_triplet()
│   ├── membership_inference.rs # Membership inference (220 lines)
│   │   ├── MembershipInferenceConfig struct
│   │   ├── MembershipScore struct
│   │   └── MembershipInference
│   │       ├── evaluate_sample()
│   │       ├── evaluate_batch()
│   │       └── high_confidence_samples()
│   ├── redaction.rs           # PII detection & redaction (210 lines)
│   │   ├── PiiRedactor struct
│   │   └── Methods:
│   │       ├── redact() → Regex-based patterns
│   │       ├── has_pii()
│   │       └── count_pii()
│   ├── quality_scorer.rs      # Quality assessment (280 lines)
│   │   ├── QualityScorerConfig struct
│   │   └── QualityScorer
│   │       ├── score_chunk()
│   │       ├── score_batch()
│   │       ├── compute_relevance()
│   │       ├── compute_accuracy()
│   │       └── compute_clarity()
│   ├── curator.rs             # Dedup + quality + PII (380 lines)
│   │   ├── CuratorConfig struct
│   │   └── Curator (multi-stage pipeline)
│   │       ├── Stage 1: Exact dedup (BLAKE3)
│   │       ├── Stage 2: MinHash LSH
│   │       ├── Stage 3: Semantic similarity
│   │       ├── PII redaction
│   │       ├── Quality scoring
│   │       └── process() → Vec<CuratedChunk>
│   ├── ingestion.rs           # KDB module generation (350 lines)
│   │   ├── IngestionConfig struct
│   │   ├── DummyEmbeddingProvider struct
│   │   └── KnowledgeIngestionPipeline
│   │       ├── ingest() → KmodPackage
│   │       ├── embed_batch()
│   │       ├── build_hnsw()
│   │       └── save_module()
│   └── kef_service.rs         # Main orchestrator (320 lines)
│       ├── KefService struct
│       └── Methods:
│           ├── extract_knowledge()
│           └── extract_knowledge_with_progress()
│
├── examples/
│   └── extract_knowledge.rs   # End-to-end example (150 lines)
│       ├── Basic extraction
│       ├── Custom configuration
│       ├── PII redaction example
│       └── Quality filtering example
│
└── tests/
    └── (module tests in each .rs file)
```

## Module Statistics

| Module | Lines | Tests | Purpose |
|--------|-------|-------|---------|
| lib.rs | 40 | 1 | Root module definitions |
| error.rs | 60 | 0 | Error handling |
| types.rs | 280 | 0 | Core data structures |
| model_scanner.rs | 320 | 3 | Model detection & inspection |
| synthetic_generator.rs | 200 | 3 | Synthetic data generation |
| activation_extractor.rs | 370 | 4 | Activation clustering |
| attention_extractor.rs | 180 | 4 | Attention triplet extraction |
| membership_inference.rs | 220 | 4 | Membership inference attacks |
| redaction.rs | 210 | 7 | PII detection & redaction |
| quality_scorer.rs | 280 | 4 | Quality scoring |
| curator.rs | 380 | 4 | Multi-stage deduplication |
| ingestion.rs | 350 | 4 | KDB module creation |
| kef_service.rs | 320 | 3 | Main orchestrator |
| **Total** | **3,610** | **45+** | |

## Data Flow Diagram

```
┌─────────────────┐
│   Model File    │
│ (GGUF/PT/ONNX)  │
└────────┬────────┘
         │
         ▼
┌─────────────────────┐
│ 1. Model Scanner    │ → ModelReport (type, params, layers)
└────────┬────────────┘
         │
    ┌────┴─────────────────────┬──────────────────┐
    │                           │                  │
    ▼                           ▼                  ▼
┌──────────────┐    ┌────────────────────┐   ┌──────────────┐
│   Synthetic  │    │    Activation      │   │  Attention   │
│  Generator   │    │    Extractor       │   │  Extractor   │
└────┬─────────┘    └────────┬───────────┘   └────┬─────────┘
     │                       │                     │
     │  Chunks              │ Activations         │ Triplets
     │                       │  + Clustering       │
     └───────────────────────┼─────────────────────┘
                             │
                    ┌────────▼────────┐
                    │  Membership     │
                    │  Inference      │
                    └────────┬────────┘
                             │
                       Chunks (all)
                             │
         ┌───────────────────▼────────────────┐
         │ 2. CURATOR                         │
         ├────────────────────────────────────┤
         │ Stage 1: Exact Dedup (BLAKE3)      │
         │ Stage 2: MinHash LSH               │
         │ Stage 3: Semantic Similarity       │
         │ Stage 4: PII Redaction             │
         │ Stage 5: Quality Scoring           │
         │ Stage 6: Threshold Filtering       │
         └────────────┬─────────────────────┘
                      │
              CuratedChunks
                      │
         ┌────────────▼──────────────┐
         │ 3. INGESTION PIPELINE     │
         ├───────────────────────────┤
         │ - Semantic Chunking       │
         │ - Batch Embedding         │
         │ - HNSW Index Building     │
         │ - Manifest Generation     │
         │ - Value Compression       │
         │ - Metadata Serialization  │
         └────────────┬──────────────┘
                      │
                   KDB Module
                   (.kmod package)
                      │
         ┌────────────▼──────────────┐
         │  4. OUTPUT & REGISTRATION │
         ├───────────────────────────┤
         │ - Save to filesystem      │
         │ - Register with KDB       │
         │ - Log to TDL              │
         │ - Publish events          │
         └───────────────────────────┘
```

## Integration Points

### Input
- Model file path (GGUF, safetensors, PyTorch, ONNX)
- Extraction methods to use
- Configuration (quality, dedup, embedding dims)

### Processing
- Model introspection
- Multi-method extraction
- Quality assessment
- PII redaction
- Deduplication (3 stages)
- Embedding generation
- Index building

### Output
- KDB modules (with HNSW index)
- Quality reports
- Provenance tracking (TDL)
- Compressed value storage
- Manifest files

## Dependency Graph

```
bonsai-kef
├── tokio (async runtime)
├── async-trait
├── serde / serde_json (serialization)
├── uuid / chrono (metadata)
├── thiserror (error handling)
├── blake3 (hashing)
├── regex (PII patterns)
├── unicode-normalization
├── ndarray (vectors)
├── rand (randomization)
├── zstd (compression)
├── crossbeam-channel (progress reporting)
│
├── bonsai-hnsw (HNSW indexing)
├── bonsai-error (error types)
│
└── [dev] tempfile, tokio-test
```

## Configuration Hierarchy

```
KefService
├── CuratorConfig
│   ├── enable_exact_dedup: bool
│   ├── enable_minhash: bool
│   ├── enable_semantic: bool
│   ├── quality_threshold: f32
│   ├── min_length: usize
│   └── max_length: usize
│
└── IngestionConfig
    ├── embedding_dim: usize
    ├── hnsw_m: usize
    ├── hnsw_ef_construction: usize
    ├── compress_values: bool
    └── batch_size: usize
```

## Error Handling Strategy

All public APIs return `Result<T>`:
- Full error context with `KefError` enum
- No panics on invalid input
- Graceful degradation where possible
- Detailed error messages

```rust
pub enum KefError {
    // IO Errors
    Io(io::Error),
    
    // Serialization
    SerdeJson(serde_json::Error),
    
    // Extraction pipeline
    ModelScan(String),
    ExtractionFailed(String),
    CurationFailed(String),
    IngestionFailed(String),
    
    // Processing errors
    EmbeddingFailed(String),
    ClusteringFailed(String),
    RedactionFailed(String),
    ScoringFailed(String),
    
    // Integration errors
    Kdb(bonsai_kdb::KdbError),
    Hnsw(bonsai_hnsw::HnswError),
    
    // Validation
    DimensionMismatch { expected, got },
    InvalidChunk(String),
    
    // Other
    Cancelled,
    Compression(String),
    Other(String),
}
```

## Testing Architecture

### Unit Tests
- Installed in each module
- ~5-10 tests per module
- Fast execution (< 100ms each)
- No external dependencies

### Integration Tests
- In examples/
- Full end-to-end flows
- Real file I/O
- Longer execution

### Test Coverage Goals
- **Target**: > 85% line coverage
- **Exclusions**: Error paths (Ok type patterns)
- **Strategy**: Focus on critical paths

## Performance Characteristics

### Memory Usage
- **Baseline**: ~50MB
- **Per Chunk**: ~1KB (text + metadata + embeddings)
- **Index**: ~8 bytes per vector dim per entry
- **Total for 10k chunks**: ~100MB

### Speed
- **Exact Dedup**: O(1) lookup
- **MinHash**: ~100 microsec per chunk
- **Semantic Search**: ~1msec per query
- **K-means Clustering**: O(n × k × d) per iteration
- **Embedding (batch)**: ~100-200 chunks/sec

### Compression
- **zstd**: ~60-70% reduction
- **Speed**: ~100MB/sec compress, ~500MB/sec decompress

## Building & Testing

```bash
# Build
cargo build -p bonsai-kef

# Run tests
cargo test -p bonsai-kef

# Run with logging
RUST_LOG=debug cargo test -p bonsai-kef -- --nocapture

# Benchmark
cargo bench -p bonsai-kef

# Example
cargo run --example extract_knowledge
```

## Code Organization Principles

1. **Modularity**: Each extraction method is independent
2. **Composition**: Main service composes all modules
3. **Error Handling**: No unwrap(), full error propagation
4. **Async First**: All APIs use async/await
5. **Zero Panics**: Validation on all inputs
6. **Testing**: Every module has unit tests
7. **Documentation**: API docs + examples + guides
8. **Type Safety**: Strong types for all concepts

## Future Extensibility

The architecture supports:
- Custom embedding models (via trait impl)
- New extraction methods (add module + update service)
- Custom quality scoring (replace QualityScorer)
- Alternative deduplication (extend Curator)
- Different output formats (extend ingestion)

