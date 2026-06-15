//! Basic model conversion example
//!
//! This example demonstrates the simplest conversion workflow:
//! GGUF → BKP conversion with progress tracking

use model_converter::*;
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Bonsai Model Converter - Basic Example\n");

    // Create conversion configuration
    let config = ConversionConfig {
        context_length: 4096,
        model_name: Some("Llama-2-7b-Custom".to_string()),
        author: Some("BonsAI".to_string()),
        license: Some("MIT".to_string()),
        description: "Example model converted to BKP".to_string(),
        compress_bkp: true,
        verify_roundtrip: true,
        ..Default::default()
    };

    println!("Configuration:");
    println!("  Context length: {}", config.context_length);
    println!("  Model name: {:?}", config.model_name);
    println!("  Compression: {}", config.compress_bkp);
    println!();

    // Example 1: Format detection
    println!("=== Format Detection ===");
    let test_paths = vec![
        "model.gguf",
        "model.safetensors",
        "model.bkp",
        "meta-llama/Llama-2-7b",
    ];

    for path in test_paths {
        match format::detect_format(Path::new(path)) {
            Ok(fmt) => println!("✓ {}: {:?}", path, fmt),
            Err(e) => println!("✗ {}: {}", path, e),
        }
    }
    println!();

    // Example 2: Create a dummy GGUF for demonstration
    println!("=== Creating Test GGUF ===");
    let test_gguf = tempfile::NamedTempFile::new()?;
    let gguf_path = test_gguf.path().to_path_buf();

    // Write GGUF magic bytes
    use std::io::Write;
    let mut file = std::fs::File::create(&gguf_path)?;
    file.write_all(b"gguf")?; // Magic
    file.write_all(&[3, 0, 0, 0])?; // Version 3
    file.write_all(&[0; 1024])?; // Dummy data

    println!("✓ Created test GGUF: {}", gguf_path.display());
    println!();

    // Example 3: Validate the test model
    println!("=== Model Validation ===");
    match validation::validate_model(&gguf_path) {
        Ok(result) => {
            println!("Format: {:?}", result.format);
            println!("Size: {} bytes", result.file_size);
            println!("Hash: {}", &result.hash[..16]);
            println!("Valid: {}", result.is_valid);
            if !result.warnings.is_empty() {
                println!("Warnings:");
                for warning in result.warnings {
                    println!("  - {}", warning);
                }
            }
        }
        Err(e) => println!("Validation error: {}", e),
    }
    println!();

    // Example 4: Attempt conversion (will show error handling)
    println!("=== Conversion Attempt ===");
    let output_path = tempfile::TempDir::new()?.path().join("output.bkp");

    println!("Converting {} → {}", gguf_path.display(), output_path.display());

    match converters::convert_gguf_to_bkp(&gguf_path, &output_path, config.clone()).await {
        Ok(()) => {
            println!("✓ Conversion successful!");

            // Verify output exists
            if output_path.exists() {
                let metadata = std::fs::metadata(&output_path)?;
                println!("✓ Output file size: {} bytes", metadata.len());
            }
        }
        Err(e) => {
            println!("✗ Conversion error: {}", e);
        }
    }
    println!();

    // Example 5: Batch conversion simulation
    println!("=== Batch Conversion Example ===");
    let input_dir = tempfile::TempDir::new()?;
    let output_dir = tempfile::TempDir::new()?;

    // Create a few dummy files
    for i in 0..3 {
        let gguf = input_dir.path().join(format!("model_{}.gguf", i));
        std::fs::write(&gguf, b"gguf")?;
    }

    println!("Created 3 dummy GGUF files in {}", input_dir.path().display());
    println!("Target output: {}", output_dir.path().display());
    println!();
    println!("Batch conversion would convert all files in parallel");
    println!("(Actual conversion skipped for this example)");
    println!();

    // Example 6: Progress reporting
    println!("=== Progress Reporting ===");
    let (reporter, mut rx) = progress::ProgressReporter::new("demo-conv");

    // Simulate progress updates
    for stage in &["reading", "converting", "writing"] {
        let progress = progress::ConversionProgress::new("demo-conv")
            .with_stage(*stage)
            .with_message(format!("Currently {}...", stage));

        reporter.report(progress)?;
        println!("✓ Reported stage: {}", stage);
    }

    // Try to receive one update
    if let Ok(Some(progress)) = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        rx.recv()
    ).await {
        println!("✓ Received progress: {} - {}", progress.stage, progress.message);
    }
    println!();

    println!("=== Example Complete ===");
    println!("\nFor real conversions, use:");
    println!("  - model_converter::convert_gguf_to_bkp()");
    println!("  - model_converter::convert_batch()");
    println!("  - CLI tool: bonsai-convert");

    Ok(())
}
