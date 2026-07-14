//! CLI demo for kef: scans a model file (or a synthetic placeholder path)
//! and runs a real extraction + curation + ingestion pass against it.

use kef::kef_service::KefService;
use kef::types::ExtractionMethod;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("model-7b.gguf"));

    let output_dir = env::temp_dir().join("kef_cli_output");

    println!("kef: extracting knowledge from {:?}", model_path);

    // If the path doesn't exist (the common case for this demo), create a
    // tiny placeholder GGUF-tagged file so the model scanner has something
    // real to detect and report on.
    if !model_path.exists() {
        std::fs::write(&model_path, b"GGUF-demo-placeholder")?;
        println!("(created placeholder model file for demo purposes)");
    }

    let service = KefService::new();
    let report = service
        .extract_knowledge(&model_path, vec![ExtractionMethod::Synthetic], &output_dir)
        .await?;

    println!(
        "Extraction complete: {} extracted, {} deduplicated, {} quality-passed, avg quality {:.3}",
        report.total_extracted, report.deduplicated, report.quality_passed, report.avg_quality
    );
    println!("Modules written: {:?}", report.modules);
    println!("Output directory: {:?}", output_dir);

    Ok(())
}
