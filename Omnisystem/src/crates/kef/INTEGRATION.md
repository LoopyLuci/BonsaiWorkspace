# BonsAI KEF Integration Guide

Integration patterns and recipes for the Knowledge Extraction Fabric within the BonsAI ecosystem.

## TDL Integration

All extracted chunks are logged to the Training Data Library for full provenance.

### Basic TDL Integration

```rust
use bonsai_tdl::{TrainingDataLibrary, Metadata};
use bonsai_kef::KefService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize TDL
    let tdl = TrainingDataLibrary::new("./training_data.db").await?;
    
    // Create a version for this extraction
    let version_id = tdl.create_version(
        "kef-extract-1.0.0",
        "kef-system",
        "Knowledge extraction via KEF",
        vec!["extracted", "synthetic"],
    ).await?;
    
    // Run extraction
    let service = KefService::new();
    let report = service.extract_knowledge(
        Path::new("model.gguf"),
        vec![ExtractionMethod::Synthetic],
        Path::new("./output"),
    ).await?;
    
    // Log results to TDL
    for module in report.modules {
        let metadata = Metadata::new()
            .with_source(&format!("kef:{}", module))
            .with_author("extraction-pipeline")
            .with_domain("knowledge")
            .with_tag("extracted");
        
        tdl.add_example(
            version_id,
            format!("Module: {}", module),
            metadata,
            report.avg_quality as f64,
        ).await?;
    }
    
    Ok(())
}
```

### TDL Query Patterns

```rust
// Find all extracted knowledge from a specific model
let examples = tdl.query_by_source("kef:extracted_model.gguf").await?;

// Get only high-quality extractions
let high_quality = examples.into_iter()
    .filter(|ex| ex.quality_score >= 0.75)
    .collect::<Vec<_>>();

// Export to training format
tdl.export_version(version_id, "output.jsonl", ExportFormat::Jsonl).await?;
```

---

## KDB Integration

Generated modules integrate seamlessly with the Knowledge Database.

### Creating KDB-Ready Modules

```rust
use bonsai_kef::{KefService, ingestion::IngestionConfig};
use bonsai_kdb::KdbManager;

let ingestion_config = IngestionConfig {
    embedding_dim: 768,
    hnsw_m: 16,
    hnsw_ef_construction: 200,
    compress_values: true,
    batch_size: 32,
};

let service = KefService::new()
    .with_ingestion_config(ingestion_config);

// Extract knowledge
let report = service.extract_knowledge(
    Path::new("model.gguf"),
    vec![ExtractionMethod::Synthetic, ExtractionMethod::Activation],
    Path::new("./kdb_modules"),
).await?;

// Register with KDB
let kdb = KdbManager::new(Path::new("./knowledge_base"))?;

for module_path in &report.modules {
    let module_dir = Path::new("./kdb_modules").join(module_path);
    kdb.load_module(&module_dir)?;
    println!("Registered module: {}", module_path);
}
```

### Querying Extracted Knowledge

```rust
use bonsai_kdb::KdbRetriever;

let retriever = KdbRetriever::new(768, 10);

// Semantic search in extracted knowledge
let query_embedding = vec![0.1; 768]; // In practice, embed your query
let results = retriever.search(&query_embedding)?;

for result in results {
    println!("Found: {} (score: {})", result.text, result.score);
}
```

---

## Universe Integration

Use KEF within the BonsAI Universe runtime for automatic knowledge management.

### Universe Event Hooks

```rust
// In your Universe hook configuration
[hooks.on_model_loaded]
action = "extract_knowledge"
extractor = "kef"
methods = ["synthetic", "activation"]
quality_threshold = 0.70

[hooks.on_extraction_complete]
action = "register_with_kdb"
auto_index = true
```

### Programmatic Integration

```rust
use bonsai_universe::{Universe, Event};
use bonsai_kef::KefService;

#[tokio::main]
async fn main() -> Result<()> {
    let universe = Universe::new().await?;
    let kef_service = KefService::new();
    
    // Listen for model loaded events
    let mut events = universe.subscribe(Event::ModelLoaded).await?;
    
    while let Some(event) = events.next().await {
        if let Event::ModelLoaded { model_path } = event {
            println!("Model loaded: {}", model_path.display());
            
            // Auto-extract knowledge
            let report = kef_service.extract_knowledge(
                &model_path,
                vec![ExtractionMethod::Synthetic],
                Path::new("./auto_extracted"),
            ).await?;
            
            // Notify universe
            universe.publish(Event::KnowledgeExtracted {
                model: model_path,
                modules: report.modules,
                quality: report.avg_quality,
            }).await?;
        }
    }
    
    Ok(())
}
```

---

## Inference Integration

Use extracted knowledge during model inference.

### Context-Enhanced Inference

```rust
use bonsai_inference::InferenceEngine;
use bonsai_kdb::KdbRetriever;

#[tokio::main]
async fn main() -> Result<()> {
    let kdb = KdbRetriever::new(768, 5);
    let inference = InferenceEngine::new("model.gguf").await?;
    
    let user_query = "What is transformer architecture?";
    
    // Retrieve relevant knowledge
    let query_embedding = inference.embed(user_query).await?;
    let context = kdb.search(&query_embedding)?;
    
    // Build context-enhanced prompt
    let context_text = context.iter()
        .map(|r| r.text.clone())
        .collect::<Vec<_>>()
        .join("\n");
    
    let prompt = format!(
        "Context:\n{}\n\nQuestion: {}\n\nAnswer:",
        context_text,
        user_query
    );
    
    // Generate response with context
    let response = inference.generate(&prompt).await?;
    println!("Response: {}", response);
    
    Ok(())
}
```

---

## Fabric Integration

Use KEF as part of the BonsAI Fabric for structured knowledge generation.

### Fabric-Aware Extraction

```rust
use bonsai_fabric::{Fabric, FabricConfig};
use bonsai_kef::KefService;

let fabric_config = FabricConfig::default();
let fabric = Fabric::new(fabric_config).await?;

let kef_service = KefService::new();

// Run extraction within fabric context
let report = kef_service.extract_knowledge(
    Path::new("model.gguf"),
    vec![ExtractionMethod::Synthetic],
    Path::new("./output"),
).await?;

// Register with fabric for downstream processing
fabric.register_knowledge_source(
    "kef",
    &report.modules,
).await?;

// Use in fabric operations
let result = fabric.perform_task("summarize_extracted_knowledge").await?;
```

---

## MCP Server Integration

Expose KEF through the Model Context Protocol.

### MCP Tool Definition

```rust
// In your MCP server implementation
use bonsai_mcp_server::{Tool, ToolInput, ToolOutput};
use bonsai_kef::KefService;

pub struct ExtractKnowledgeTool {
    service: KefService,
}

impl Tool for ExtractKnowledgeTool {
    fn name(&self) -> &str {
        "extract_knowledge"
    }
    
    fn description(&self) -> &str {
        "Extract knowledge from AI models using multiple methods"
    }
    
    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "model_path": {"type": "string"},
                "methods": {
                    "type": "array",
                    "items": {"type": "string"},
                },
                "output_dir": {"type": "string"},
            },
            "required": ["model_path", "methods", "output_dir"],
        })
    }
    
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let model_path = input.get_string("model_path")?;
        let methods: Vec<ExtractionMethod> = input.get_array("methods")?
            .into_iter()
            .map(|m| ExtractionMethod::from_str(&m))
            .collect::<Result<_>>()?;
        let output_dir = input.get_string("output_dir")?;
        
        let report = self.service.extract_knowledge(
            Path::new(&model_path),
            methods,
            Path::new(&output_dir),
        ).await?;
        
        Ok(ToolOutput::json(json!({
            "status": "success",
            "modules": report.modules,
            "quality": report.avg_quality,
            "duration_secs": report.duration_secs,
        })))
    }
}
```

---

## Skill Integration

Register KEF as a BonsAI Skill.

### Skill Definition

```yaml
# kef-skill.yaml
name: extract-knowledge
version: 1.0.0
description: Extract knowledge from AI models
category: knowledge

triggers:
  - event: model.loaded
  - command: kef extract

parameters:
  model_path:
    type: string
    description: Path to model file
  methods:
    type: array
    items:
      type: string
      enum: [synthetic, activation, attention, membership_inference]
    description: Extraction methods to use
  output_dir:
    type: string
    description: Output directory for KDB modules

outputs:
  modules:
    type: array
    items: string
  quality_score:
    type: number
  duration_secs:
    type: number
```

### Skill Implementation

```rust
use bonsai_skills::{Skill, SkillContext, SkillResult};

pub struct ExtractKnowledgeSkill {
    service: KefService,
}

#[async_trait::async_trait]
impl Skill for ExtractKnowledgeSkill {
    async fn execute(&self, ctx: SkillContext) -> SkillResult {
        let model_path: String = ctx.param("model_path")?;
        let methods: Vec<String> = ctx.param("methods")?;
        let output_dir: String = ctx.param("output_dir")?;
        
        let methods = methods.into_iter()
            .map(|m| ExtractionMethod::from_str(&m))
            .collect::<Result<_, _>>()?;
        
        let report = self.service.extract_knowledge(
            Path::new(&model_path),
            methods,
            Path::new(&output_dir),
        ).await?;
        
        ctx.set_output("modules", serde_json::to_value(&report.modules)?);
        ctx.set_output("quality_score", report.avg_quality);
        ctx.set_output("duration_secs", report.duration_secs);
        
        ctx.log(format!("Extracted {} modules", report.modules.len()));
        Ok(())
    }
}
```

---

## CI Integration

Use KEF in BonsAI CI pipelines.

### CI Configuration

```toml
# In .bonsai-ci/config.toml

[[jobs]]
name = "extract-knowledge"
description = "Extract knowledge from model artifacts"

[jobs.steps]
scan = "model-scanner"
extract = {
    methods = ["synthetic", "activation"]
    quality_threshold = 0.7
}
register = "kdb-register"
export = {
    format = "jsonl"
    dest = "training_data.jsonl"
}
```

### CI Script Example

```bash
#!/bin/bash
# scripts/extract_knowledge.sh

MODEL_PATH=$1
OUTPUT_DIR=${2:-.bonsai-ci/artifacts}

echo "📦 Extracting knowledge from $MODEL_PATH..."

cargo run --release -p bonsai-kef --example extract_knowledge -- \
    --model "$MODEL_PATH" \
    --methods synthetic activation \
    --output "$OUTPUT_DIR" \
    --quality-threshold 0.70

if [ $? -eq 0 ]; then
    echo "✅ Knowledge extraction successful"
    echo "📚 Modules saved to: $OUTPUT_DIR"
else
    echo "❌ Knowledge extraction failed"
    exit 1
fi
```

---

## Testing Strategy

### Unit Tests

```bash
cargo test -p bonsai-kef
```

### Integration Tests

```bash
# Test with real model
cargo test -p bonsai-kef --features integration -- --nocapture

# Test TDL integration
cargo test -p bonsai-kef test_tdl_integration -- --nocapture
```

### Benchmarks

```bash
# Run performance benchmarks
cargo bench -p bonsai-kef

# Profile deduplication overhead
PROFILING=1 cargo test -p bonsai-kef test_curator_performance
```

---

## Deployment

### Docker Integration

```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p bonsai-kef

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/bonsai-kef /usr/local/bin/
ENTRYPOINT ["bonsai-kef"]
```

### Kubernetes Job

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: kef-extraction
spec:
  template:
    spec:
      containers:
      - name: kef
        image: bonsai:latest
        args: ["extract", "--model", "/models/model.gguf", "--output", "/output"]
        volumeMounts:
        - name: models
          mountPath: /models
        - name: output
          mountPath: /output
      volumes:
      - name: models
        persistentVolumeClaim:
          claimName: model-storage
      - name: output
        persistentVolumeClaim:
          claimName: extraction-output
      restartPolicy: Never
```

---

## Performance Tuning

### Memory Optimization

```rust
let config = CuratorConfig {
    enable_semantic: false,  // Disable expensive semantic dedup
    enable_minhash: true,    // Lightweight LSH
    ..Default::default()
};

let service = KefService::new()
    .with_curator_config(config);
```

### Throughput Optimization

```rust
let ingestion_config = IngestionConfig {
    batch_size: 256,         // Larger batches
    hnsw_ef_construction: 100, // Faster (lower recall)
    ..Default::default()
};

let service = KefService::new()
    .with_ingestion_config(ingestion_config);
```

### Latency Optimization

```rust
let ingestion_config = IngestionConfig {
    batch_size: 16,          // Smaller batches
    hnsw_ef_construction: 400, // Better recall
    compress_values: false,  // Trade space for speed
    ..Default::default()
};
```

---

## Monitoring & Observability

### Logging

```rust
use tracing::{info, warn, error};

// Automatically instrumented logs
info!("KEF: Starting extraction from {:?}", model_path);
warn!("KEF: Low quality samples detected");
error!("KEF: Extraction failed: {}", error);
```

### Metrics

```rust
// Export metrics in Prometheus format
let metrics = MetricsCollector::new();
metrics.record_extraction_duration(report.duration_secs);
metrics.record_quality_score(report.avg_quality);
metrics.record_module_count(report.modules.len());
```

---

## Troubleshooting

### Common Issues

**Issue**: Low quality scores
- **Solution**: Decrease `quality_threshold` in config, or check model fit for task

**Issue**: Memory exhaustion
- **Solution**: Reduce `batch_size`, disable semantic dedup, limit `max_activations`

**Issue**: Slow extraction
- **Solution**: Reduce `hnsw_ef_construction`, disable semantic dedup, use larger batches

**Issue**: High deduplication rate
- **Solution**: Check for repetitive data sources, adjust thresholds, use different seed topics

---

## Future Enhancements

- [ ] Custom embedding models via trait
- [ ] Distributed extraction across workers
- [ ] Real-time streaming ingestion
- [ ] Interactive quality tuning
- [ ] Federated knowledge federation
- [ ] Active learning for sampling
- [ ] Multi-GPU support

