# Bonsai KEF API Reference

Complete API documentation for the Knowledge Extraction Fabric.

## Core Types

### ExtractionMethod

Enum specifying which extraction methods to use.

```rust
pub enum ExtractionMethod {
    Synthetic,              // Generate synthetic explanations
    Activation,             // Extract activation vectors
    Attention,              // Extract attention patterns
    MembershipInference,    // Membership inference
}
```

**Display/FromStr:**
```rust
impl Display for ExtractionMethod
impl FromStr for ExtractionMethod
```

---

### QualityScores

Quality assessment dimensions.

```rust
pub struct QualityScores {
    pub relevance: f32,        // 0.0-1.0
    pub accuracy: f32,         // 0.0-1.0
    pub clarity: f32,          // 0.0-1.0
    pub uniqueness: f32,       // 0.0-1.0
    pub aggregate: f32,        // 0.0-1.0 (weighted average)
}
```

**Methods:**
- `new() -> Self` - Create default scores
- `update_aggregate(&mut self)` - Recalculate aggregate from components

---

### CuratedChunk

A single knowledge chunk after curation.

```rust
pub struct CuratedChunk {
    pub content: String,
    pub quality_scores: QualityScores,
    pub pii_redacted: bool,
    pub extraction_method: String,
    pub source_model: String,
    pub extracted_at: DateTime<Utc>,
    pub tags: Vec<String>,
}
```

**Methods:**
- `new(content, source_model, method) -> Self`
- `with_tag(tag) -> Self` - Add a tag

---

### KmodPackage

A generated Knowledge Database module.

```rust
pub struct KmodPackage {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub domain: String,
    pub entry_count: usize,
    pub embedding_dim: usize,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub extraction_methods: Vec<String>,
    pub avg_quality_score: f32,
}
```

**Methods:**
- `new(name, domain) -> Self`
- `with_description(desc) -> Self`
- `with_embedding_dim(dim) -> Self`

---

### ExtractionReport

Summary of an extraction operation.

```rust
pub struct ExtractionReport {
    pub total_extracted: usize,
    pub deduplicated: usize,
    pub pii_redacted: usize,
    pub quality_passed: usize,
    pub avg_quality: f32,
    pub methods_used: Vec<String>,
    pub duration_secs: f64,
    pub modules: Vec<String>,
    pub errors: Vec<String>,
}
```

**Methods:**
- `new() -> Self`
- `dedup_ratio() -> f64` - Returns (deduplicated / total_extracted)
- `quality_pass_ratio() -> f64` - Returns (quality_passed / deduplicated)

---

## Model Scanner

Detects and introspects model files.

### ModelType

```rust
pub enum ModelType {
    Llm,       // Large Language Model
    Embedding, // Text embedding
    Vision,    // Vision transformer
    Moe,       // Mixture of Experts
    Other,     // Unknown type
}
```

### ModelReport

```rust
pub struct ModelReport {
    pub model_type: ModelType,
    pub parameter_count: u64,
    pub layers: usize,
    pub hidden_size: usize,
    pub context_window: usize,
    pub quantization: Option<String>,
    pub applicable_methods: Vec<ExtractionMethod>,
    pub file_format: String,
    pub model_name: String,
    pub vocab_size: usize,
}
```

### ModelScanner

```rust
impl ModelScanner {
    pub async fn scan(path: &Path) -> Result<ModelReport>
}
```

**Supported Formats:**
- GGUF (`.gguf`)
- safetensors (`.safetensors`)
- PyTorch (`.pt`, `.pth`)
- ONNX (`.onnx`)

---

## Extraction Methods

### SyntheticDataGenerator

Generates synthetic training data from model outputs.

```rust
pub struct SyntheticGeneratorConfig {
    pub temperature: f32,              // 0.0-2.0
    pub max_tokens: usize,             // Generation limit
    pub samples_per_topic: usize,      // Samples per topic
    pub topics: Vec<String>,           // Topics to expand
}

pub struct SyntheticDataGenerator { ... }
```

**Methods:**
```rust
impl SyntheticDataGenerator {
    pub fn new(config: SyntheticGeneratorConfig) -> Self
    pub async fn generate_from_topics(&self) -> Result<Vec<String>>
    pub async fn generate_from_vocabulary(&self, vocab_size: usize) 
        -> Result<Vec<String>>
    pub async fn generate_beam_search(&self, topic: &str, beam_width: usize)
        -> Result<Vec<String>>
}
```

---

### ActivationExtractor

Extracts and clusters neuron activation patterns.

```rust
pub struct ActivationExtractorConfig {
    pub sparsity_threshold: f32,       // 0.0-1.0, default 0.3
    pub num_clusters: usize,           // Default 16
    pub max_activations: usize,        // Default 10000
    pub target_layers: Vec<usize>,     // Empty = all
}

pub struct ActivationSample {
    pub layer: usize,
    pub vector: Vec<f32>,
    pub sparsity: f32,
    pub input_text: String,
}

pub struct ActivationCluster {
    pub centroid: Vec<f32>,
    pub members: Vec<ActivationSample>,
    pub description: Option<String>,
}

pub struct ActivationExtractor { ... }
```

**Methods:**
```rust
impl ActivationExtractor {
    pub fn new(config: ActivationExtractorConfig) -> Self
    pub fn add_sample(&mut self, sample: ActivationSample) -> Result<()>
    pub fn cluster(&self) -> Result<Vec<ActivationCluster>>
    pub fn samples(&self) -> &[ActivationSample]
}

impl ActivationCluster {
    pub fn new(samples: Vec<ActivationSample>) -> Result<Self>
    pub fn distance_to_centroid(&self, vector: &[f32]) -> Result<f32>
}
```

---

### AttentionExtractor

Extracts knowledge triplets from attention patterns.

```rust
pub struct AttentionExtractorConfig {
    pub weight_threshold: f32,         // Default 0.5
    pub max_triplets: usize,           // Default 1000
    pub target_heads: Vec<usize>,      // Empty = all
}

pub struct KnowledgeTriplet {
    pub subject: String,
    pub relation: String,
    pub object: String,
    pub confidence: f32,               // 0.0-1.0
    pub attention_weights: Vec<f32>,
}

pub struct AttentionExtractor { ... }
```

**Methods:**
```rust
impl AttentionExtractor {
    pub fn new(config: AttentionExtractorConfig) -> Self
    pub fn extract_from_attention(
        &mut self,
        tokens: &[String],
        attention_weights: &[Vec<f32>],
    ) -> Result<()>
    pub fn add_triplet(&mut self, triplet: KnowledgeTriplet) -> Result<()>
    pub fn triplets(&self) -> &[KnowledgeTriplet]
    pub fn validate_triplet(&self, triplet: &KnowledgeTriplet) -> bool
}
```

---

### MembershipInference

Identifies likely training data through inference attacks.

```rust
pub struct MembershipInferenceConfig {
    pub loss_threshold: f32,           // Default 2.0
    pub probability_threshold: f32,    // Default 0.1
    pub max_samples: usize,            // Default 10000
    pub batch_size: usize,             // Default 32
}

pub struct MembershipScore {
    pub text: String,
    pub loss: f32,
    pub probability: f32,
    pub confidence: f32,               // 0.0-1.0
    pub likely_in_training: bool,
}

pub struct MembershipInference { ... }
```

**Methods:**
```rust
impl MembershipInference {
    pub fn new(config: MembershipInferenceConfig) -> Self
    pub async fn evaluate_sample(&mut self, text: &str) 
        -> Result<MembershipScore>
    pub async fn evaluate_batch(&mut self, texts: &[&str])
        -> Result<Vec<MembershipScore>>
    pub fn high_confidence_samples(&self) -> Vec<&MembershipScore>
    pub fn scores(&self) -> &[MembershipScore]
}
```

---

## Curation

### PiiRedactor

Detects and redacts personally identifiable information.

```rust
pub struct PiiRedactor { ... }
```

**Methods:**
```rust
impl PiiRedactor {
    pub fn new() -> Self
    pub fn whitelist_entity(&mut self, entity: String)
    pub fn redact(&self, text: &str) -> String
    pub fn has_pii(&self, text: &str) -> bool
    pub fn count_pii(&self, text: &str) -> usize
}
```

**Detected PII Types:**
- Emails: `[EMAIL]`
- Phone numbers: `[PHONE]`
- Credit cards: `[CREDIT_CARD]`
- Social Security Numbers: `[SSN]`
- IP addresses: `[IP_ADDRESS]`

---

### QualityScorer

Computes quality dimensions for knowledge chunks.

```rust
pub struct QualityScorerConfig {
    pub min_quality: f32,              // Threshold, default 0.65
    pub relevance_weight: f32,         // Default 0.25
    pub accuracy_weight: f32,          // Default 0.35
    pub clarity_weight: f32,           // Default 0.25
    pub uniqueness_weight: f32,        // Default 0.15
}

pub struct QualityScorer { ... }
```

**Methods:**
```rust
impl QualityScorer {
    pub fn new(config: QualityScorerConfig) -> Self
    pub async fn score_chunk(&self, content: &str, uniqueness: f32)
        -> Result<QualityScores>
    pub async fn score_batch(&self, chunks: &[&str])
        -> Result<Vec<QualityScores>>
    pub fn passes_threshold(&self, score: &QualityScores) -> bool
}
```

---

### Curator

Multi-stage deduplication and quality filtering.

```rust
pub struct CuratorConfig {
    pub enable_exact_dedup: bool,      // Stage 1, default true
    pub enable_minhash: bool,          // Stage 2, default true
    pub enable_semantic: bool,         // Stage 3, default true
    pub quality_threshold: f32,        // Default 0.65
    pub min_length: usize,             // Default 20
    pub max_length: usize,             // Default 10000
}

pub struct Curator { ... }
```

**Methods:**
```rust
impl Curator {
    pub fn new(config: CuratorConfig) -> Self
    pub async fn process(&mut self, chunks: Vec<String>)
        -> Result<Vec<CuratedChunk>>
}
```

**Processing Stages:**
1. Length validation
2. Exact hash deduplication
3. MinHash LSH similarity
4. PII redaction
5. Quality scoring
6. Threshold filtering

---

## Ingestion

### IngestionConfig

```rust
pub struct IngestionConfig {
    pub embedding_dim: usize,          // Default 768
    pub hnsw_m: usize,                 // Default 16
    pub hnsw_ef_construction: usize,   // Default 200
    pub compress_values: bool,         // Default true
    pub batch_size: usize,             // Default 32
}
```

### DummyEmbeddingProvider

Placeholder embedder (for testing/demo).

```rust
pub struct DummyEmbeddingProvider { ... }

impl DummyEmbeddingProvider {
    pub fn new(dim: usize) -> Self
    pub fn embed(&self, text: &str) -> Result<Vec<f32>>
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>
    pub fn dimension(&self) -> usize
}
```

### KnowledgeIngestionPipeline

Converts curated chunks into KDB modules.

```rust
pub struct KnowledgeIngestionPipeline { ... }
```

**Methods:**
```rust
impl KnowledgeIngestionPipeline {
    pub fn new(config: IngestionConfig, embedder: DummyEmbeddingProvider) -> Self
    pub async fn ingest(
        &self,
        chunks: Vec<CuratedChunk>,
        module_name: &str,
        domain: &str,
    ) -> Result<KmodPackage>
    pub async fn save_module(
        &self,
        module: &KmodPackage,
        chunks: &[CuratedChunk],
        output_dir: &Path,
    ) -> Result<()>
}
```

---

## Main Service

### KefService

Main orchestrator for knowledge extraction.

```rust
pub struct KefService { ... }
```

**Methods:**
```rust
impl KefService {
    pub fn new() -> Self
    pub fn with_curator_config(self, config: CuratorConfig) -> Self
    pub fn with_ingestion_config(self, config: IngestionConfig) -> Self
    
    pub async fn extract_knowledge(
        &self,
        model_path: &Path,
        methods: Vec<ExtractionMethod>,
        output_dir: &Path,
    ) -> Result<ExtractionReport>
    
    pub async fn extract_knowledge_with_progress(
        &self,
        model_path: &Path,
        methods: Vec<ExtractionMethod>,
        output_dir: &Path,
    ) -> Result<(ExtractionReport, crossbeam_channel::Receiver<String>)>
}
```

---

## Error Types

All functions return `Result<T>` where `T` is the success type.

```rust
pub enum KefError {
    Io(io::Error),
    SerdeJson(serde_json::Error),
    ModelScan(String),
    UnsupportedModel(String),
    ExtractionFailed(String),
    EmbeddingFailed(String),
    ClusteringFailed(String),
    CurationFailed(String),
    IngestionFailed(String),
    RedactionFailed(String),
    ScoringFailed(String),
    Kdb(bonsai_kdb::KdbError),
    Tdl(String),
    Hnsw(bonsai_hnsw::HnswError),
    InvalidChunk(String),
    DimensionMismatch { expected: usize, got: usize },
    Cancelled,
    Compression(String),
    ModelLoading(String),
    UnicodeNormalization,
    Other(String),
}

pub type Result<T> = std::result::Result<T, KefError>;
```

---

## Usage Examples

### Simple Extraction

```rust
use bonsai_kef::{KefService, ExtractionMethod};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = KefService::new();
    
    let report = service.extract_knowledge(
        Path::new("model.gguf"),
        vec![ExtractionMethod::Synthetic],
        Path::new("./output"),
    ).await?;
    
    println!("Extracted {} chunks with {:.2}% quality", 
        report.deduplicated, 
        report.avg_quality * 100.0
    );
    Ok(())
}
```

### Custom Configuration

```rust
use bonsai_kef::{KefService, curator::CuratorConfig, ingestion::IngestionConfig};

let curator_config = CuratorConfig {
    quality_threshold: 0.75,
    enable_semantic: true,
    ..Default::default()
};

let ingestion_config = IngestionConfig {
    embedding_dim: 1024,
    hnsw_m: 32,
    ..Default::default()
};

let service = KefService::new()
    .with_curator_config(curator_config)
    .with_ingestion_config(ingestion_config);
```

### With Progress Reporting

```rust
let (report, rx) = service.extract_knowledge_with_progress(
    model_path,
    methods,
    output_dir,
).await?;

for msg in rx.iter() {
    println!("Progress: {}", msg);
}
```

---

## Threading & Async

All public APIs are async-friendly using `tokio`:

```rust
#[tokio::main]
async fn main() {
    // All KEF APIs are async
    let result = service.extract_knowledge(...).await?;
}
```

For blocking code, use `tokio::task::block_in_place`:

```rust
let report = tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async {
        service.extract_knowledge(...).await
    })
})?;
```

---

## Performance Tips

1. **Batch Extraction**: Process multiple models sequentially to amortize startup cost
2. **Compression**: Use `zstd` compression for values (10x smaller, minimal overhead)
3. **Embedding Batch Size**: Match to GPU/CPU capacity (32-256)
4. **HNSW Parameters**: Higher `m` (32-64) for better recall, slower build
5. **Quality Threshold**: 0.65 for recall, 0.75+ for precision

---

## Version History

- **0.1.0** (2026-06-01): Initial release
  - All extraction methods
  - Multi-stage deduplication
  - PII redaction
  - Quality scoring
  - KDB integration

