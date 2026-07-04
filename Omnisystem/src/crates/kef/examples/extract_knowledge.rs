//! Example: Full end-to-end knowledge extraction pipeline
//!
//! This example demonstrates:
//! - Model scanning
//! - Extraction from multiple methods
//! - Curation and quality filtering
//! - KDB module generation
//! - Reporting

use kef::{
    curator::CuratorConfig,
    ingestion::IngestionConfig,
    ExtractionMethod,
    KefService,
};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Configuration
    let model_path = Path::new("models/bonsai-8b.gguf");
    let output_dir = Path::new("./extracted_knowledge");

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    println!("╔════════════════════════════════════════════════╗");
    println!("║   Bonsai Knowledge Extraction Fabric (KEF)     ║");
    println!("║              Example Pipeline                  ║");
    println!("╚════════════════════════════════════════════════╝");
    println!();

    // Configure curator for high-quality output
    let curator_config = CuratorConfig {
        enable_exact_dedup: true,
        enable_minhash: true,
        enable_semantic: true,
        quality_threshold: 0.65,
        min_length: 20,
        max_length: 5000,
    };

    // Configure ingestion for 768-dim embeddings (common size)
    let ingestion_config = IngestionConfig {
        embedding_dim: 768,
        hnsw_m: 16,
        hnsw_ef_construction: 200,
        compress_values: true,
        batch_size: 32,
    };

    // Create service with custom config
    let service = KefService::new()
        .with_curator_config(curator_config)
        .with_ingestion_config(ingestion_config);

    // Specify extraction methods
    let methods = vec![
        ExtractionMethod::Synthetic,
        ExtractionMethod::Activation,
        ExtractionMethod::Attention,
        ExtractionMethod::MembershipInference,
    ];

    println!("📦 Extraction Configuration:");
    println!("  Model: {}", model_path.display());
    println!("  Methods: {}", methods.iter()
        .map(|m| m.to_string())
        .collect::<Vec<_>>()
        .join(", "));
    println!("  Output: {}", output_dir.display());
    println!("  Quality Threshold: 0.65");
    println!("  Embedding Dim: 768");
    println!();

    // Run extraction
    println!("⚡ Starting extraction...");
    match service
        .extract_knowledge(model_path, methods, output_dir)
        .await
    {
        Ok(report) => {
            println!();
            println!("✅ Extraction Complete!");
            println!();
            println!("📊 Statistics:");
            println!("  Total Extracted: {}", report.total_extracted);
            println!("  After Dedup: {}", report.deduplicated);
            println!("  PII Redacted: {}", report.pii_redacted);
            println!("  Quality Passed: {}", report.quality_passed);
            println!("  Dedup Ratio: {:.2}%", report.dedup_ratio() * 100.0);
            println!("  Quality Pass Rate: {:.2}%", report.quality_pass_ratio() * 100.0);
            println!("  Average Quality: {:.3}", report.avg_quality);
            println!("  Duration: {:.2}s", report.duration_secs);
            println!();

            if !report.modules.is_empty() {
                println!("📚 Generated Modules:");
                for (i, module) in report.modules.iter().enumerate() {
                    println!("  {}. {}", i + 1, module);
                }
            }

            if !report.errors.is_empty() {
                println!();
                println!("⚠️  Errors Encountered:");
                for error in &report.errors {
                    println!("  - {}", error);
                }
            }

            println!();
            println!("Module saved to: {}", output_dir.display());
        }
        Err(e) => {
            println!("❌ Extraction failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

// Optional: Example of custom extraction methods
#[allow(dead_code)]
async fn example_custom_extraction() -> Result<(), Box<dyn std::error::Error>> {
    use kef::synthetic_generator::{SyntheticDataGenerator, SyntheticGeneratorConfig};

    // Configure topic-based generation
    let config = SyntheticGeneratorConfig {
        temperature: 0.7,
        max_tokens: 512,
        samples_per_topic: 5,
        topics: vec![
            "artificial intelligence".to_string(),
            "machine learning".to_string(),
            "knowledge representation".to_string(),
            "neural networks".to_string(),
            "natural language processing".to_string(),
        ],
    };

    let generator = SyntheticDataGenerator::new(config);

    println!("Generating synthetic data from topics...");
    let chunks = generator.generate_from_topics().await?;
    println!("Generated {} chunks", chunks.len());

    Ok(())
}

// Optional: Example of quality filtering
#[allow(dead_code)]
async fn example_quality_filtering() -> Result<(), Box<dyn std::error::Error>> {
    use kef::quality_scorer::{QualityScorer, QualityScorerConfig};

    let scorer = QualityScorer::new(QualityScorerConfig::default());

    let texts = vec![
        "Machine learning is a subset of artificial intelligence.",
        "Deep learning models have revolutionized the field of AI.",
        "!!!! GARBAGE !!!! xxx !!!",
    ];

    println!("Scoring chunks for quality...");
    for text in &texts {
        let score = scorer.score_chunk(text, 0.8).await?;
        println!("  '{}': {:.3}", text, score.aggregate);
    }

    Ok(())
}

// Optional: Example of PII redaction
#[allow(dead_code)]
fn example_pii_redaction() {
    use kef::redaction::PiiRedactor;

    let redactor = PiiRedactor::new();

    let text = "Contact John Doe at john.doe@example.com or 555-123-4567 for details.";
    let redacted = redactor.redact(text);

    println!("Original:  {}", text);
    println!("Redacted:  {}", redacted);
}
