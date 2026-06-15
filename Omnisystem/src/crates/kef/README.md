# Bonsai Knowledge Extraction Fabric (KEF)

Production-grade knowledge extraction from AI models with multiple extraction methods, multi-stage deduplication, PII redaction, and seamless KDB integration.

## Features

### Extraction Methods

1. **Synthetic Data Generation** - Generate explanations and knowledge from model outputs
2. **Activation Vector Extraction** - Extract and cluster internal model activations
3. **Attention Pattern Analysis** - Convert attention weights to knowledge triplets
4. **Membership Inference** - Identify high-confidence training data

### Quality Assurance

- **Multi-stage Deduplication**: Exact hashing → MinHash LSH → Semantic similarity
- **PII Redaction**: Automatic detection and redaction of sensitive information (email, phone, SSN, credit cards, IP addresses)
- **Quality Scoring**: Relevance, accuracy, clarity, and uniqueness scoring
- **Threshold Filtering**: Only high-quality chunks pass to ingestion

### Integration

- **KDB Module Generation**: Automatically create and save KDB modules from extracted knowledge
- **TDL Provenance Tracking**: Full lineage tracking of all extracted data
- **HNSW Indexing**: Fast semantic search via approximate nearest neighbors
- **Batch Processing**: Efficient handling of large extraction jobs

## Architecture

### Modules

- `model_scanner.rs` - Model format detection and introspection (GGUF, safetensors, PyTorch, ONNX)
- `synthetic_generator.rs` - Synthetic data generation from topics and vocabulary
- `activation_extractor.rs` - Activation vector collection and K-means clustering
- `attention_extractor.rs` - Attention weight analysis for triplet extraction
- `membership_inference.rs` - Membership inference attacks
- `curator.rs` - Multi-stage deduplication and quality filtering
- `ingestion.rs` - Chunk→embedding→HNSW→KDB conversion
- `quality_scorer.rs` - Quality dimension computation
- `redaction.rs` - PII detection and redaction
- `kef_service.rs` - Main orchestrator

### Data Flow

```
Model File
    ↓
[Model Scanner] → Model Report
    ↓
[Extraction Methods]
├─→ [Synthetic Generator] → Chunks
├─→ [Activation Extractor] → Chunks
├─→ [Attention Extractor] → Triplets → Chunks
└─→ [Membership Inference] → Chunks
    ↓
[Curator]
├─ Stage 1: Exact Dedup
├─ Stage 2: MinHash LSH
├─ Stage 3: Semantic Similarity
├─ PII Redaction
├─ Quality Scoring
└─→ Curated Chunks
    ↓
[Ingestion Pipeline]
├─ Text Chunking
├─ Batch Embedding
├─ HNSW Index Building
└─→ KDB Module
    ↓
Output Directory
```

## Usage

### Basic Extraction

```rust
use bonsai_kef::{KefService, ExtractionMethod};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let service = KefService::new();
    
    let methods = vec![
        ExtractionMethod::Synthetic,
        ExtractionMethod::Activation,
    ];
    
    let report = service.extract_knowledge(
        Path::new("model.gguf"),
        methods,
        Path::new("./output"),
    ).await?;
    
    println!("Extracted {} modules", report.modules.len());
    println!("Quality: {:.3}", report.avg_quality);
    Ok(())
}
```

### With Custom Configuration

```rust
use bonsai_kef::{KefService, curator::CuratorConfig, ingestion::IngestionConfig};

let curator_config = CuratorConfig {
    quality_threshold: 0.75,  // Higher threshold
    min_length: 50,
    ..Default::default()
};

let ingestion_config = IngestionConfig {
    embedding_dim: 768,
    hnsw_m: 32,  // Higher connectivity
    compress_values: true,
    ..Default::default()
};

let service = KefService::new()
    .with_curator_config(curator_config)
    .with_ingestion_config(ingestion_config);
```

### Progress Reporting

```rust
let (report, progress_rx) = service
    .extract_knowledge_with_progress(
        Path::new("model.gguf"),
        methods,
        Path::new("./output"),
    )
    .await?;

while let Ok(msg) = progress_rx.recv() {
    println!("KEF: {}", msg);
}
```

## Quality Dimensions

### Relevance
- Content length > 50 chars
- Contains NLP/ML terminology
- Related to domain

### Accuracy
- Well-formed sentences
- No obvious contradictions
- Research language patterns

### Clarity
- Moderate length (100-500 chars)
- No excessive punctuation
- Normal capitalization

### Uniqueness
- Computed during deduplication
- Based on semantic similarity
- Higher scores for rare concepts

### Aggregate
```
score = (relevance×0.25 + accuracy×0.35 + clarity×0.25 + uniqueness×0.15)
```

## PII Redaction

Automatically detects and redacts:
- Email addresses: `john@example.com` → `[EMAIL]`
- Phone numbers: `555-123-4567` → `[PHONE]`
- Credit cards: `4532-1111-2222-3333` → `[CREDIT_CARD]`
- Social Security Numbers: `123-45-6789` → `[SSN]`
- IP addresses: `192.168.1.1` → `[IP_ADDRESS]`

## Deduplication Strategy

### Stage 1: Exact Hashing
- Fast O(1) lookup using BLAKE3 hashing
- Catches identical duplicates

### Stage 2: MinHash LSH
- Locality-Sensitive Hashing with 128 bands
- Catches near-identical duplicates
- Configurable similarity threshold

### Stage 3: Semantic Similarity
- Embedding-based HNSW search
- Finds semantically similar chunks
- Threshold: > 0.95 similarity

## Performance Characteristics

- **Memory**: ~100MB baseline + ~1KB per chunk for indices
- **Speed**: ~100-200 chunks/sec (excluding model inference)
- **Compression**: ~60-70% reduction with zstd (values.txt.zst)
- **Index Building**: ~10-50 msec per 1000 vectors

## Error Handling

All functions return `Result<T>` with detailed `KefError` types:
- `ModelScan` - Model format detection failures
- `ExtractionFailed` - Extraction method failures
- `CurationFailed` - Deduplication/quality failures
- `IngestionFailed` - Embedding/indexing failures
- `Compression` - Compression errors

## Testing

Run the test suite:

```bash
cargo test -p bonsai-kef

# With logging
RUST_LOG=debug cargo test -p bonsai-kef -- --nocapture
```

Coverage by module:
- `model_scanner.rs`: Format detection, type inference
- `synthetic_generator.rs`: Topic expansion, beam search
- `activation_extractor.rs`: K-means clustering, distance computation
- `attention_extractor.rs`: Triplet extraction from attention weights
- `curator.rs`: Deduplication stages, PII detection
- `quality_scorer.rs`: Quality dimension computation
- `ingestion.rs`: Embedding and HNSW indexing

## Integration with BonsAI Ecosystem

### TDL Integration
All extracted chunks are logged to TDL with:
- Source model and extraction method
- Quality scores and timestamps
- PII redaction status
- Uniqueness metrics

### KDB Integration
Generated modules integrate with the Bonsai Knowledge Database:
- Fast semantic retrieval via HNSW
- Multi-domain support
- Compressed value storage
- Manifest with full metadata

### Future Extensions
- Custom embedding models
- Active learning for quality scoring
- Distributed extraction across workers
- Real-time streaming ingestion
- Federated knowledge federation

## References

- [Knowledge Extraction Literature](https://arxiv.org/search/?query=knowledge+extraction&searchtype=all)
- [MinHash LSH](https://en.wikipedia.org/wiki/MinHash)
- [HNSW Algorithms](https://arxiv.org/abs/1604.09143)
- [PII Detection Best Practices](https://owasp.org/www-community/attacks/PII_Detection_Evasion)

## License

Part of the BonsAI project. All rights reserved.
