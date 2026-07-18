//! Example: Create and load a BKP package
//!
//! This demonstrates the complete workflow of:
//! 1. Creating a BKP package with BkpBuilder
//! 2. Loading and inspecting it with BkpLoader

use bkp::*;
use std::io::Write;

fn main() -> anyhow::Result<()> {
    println!("Bonsai BKP Package Builder & Loader Example\n");

    // Create temporary files for the example
    let temp_dir = tempfile::TempDir::new()?;
    let bkp_output = temp_dir.path().join("example.bkp");

    // Example 1: Build a BKP package
    println!("=== Building BKP Package ===\n");

    // Create dummy base model file
    let model_path = temp_dir.path().join("model.gguf");
    {
        let mut file = std::fs::File::create(&model_path)?;
        file.write_all(b"gguf")?;
        file.write_all(&vec![0; 10000])?; // Dummy model data
    }
    println!("✓ Created dummy GGUF model: {}", model_path.display());

    // Create dummy KMOD module
    let kmod_path = temp_dir.path().join("knowledge.kmod");
    {
        let mut file = std::fs::File::create(&kmod_path)?;
        file.write_all(b"kmod")?;
        file.write_all(&vec![0; 5000])?;
    }
    println!("✓ Created dummy KMOD module: {}", kmod_path.display());

    // Create dummy adapter
    let adapter_path = temp_dir.path().join("adapter.safetensors");
    {
        let mut file = std::fs::File::create(&adapter_path)?;
        file.write_all(&vec![0; 2000])?;
    }
    println!("✓ Created dummy adapter: {}\n", adapter_path.display());

    // Build the BKP
    println!("Building BKP package...");
    let mut builder = BkpBuilder::new("example-model", "1.0.0")?;

    builder.add_base_model(&model_path)?;
    println!("  ✓ Added base model");

    builder.add_kmod_module(&kmod_path, "knowledge")?;
    println!("  ✓ Added KMOD module: knowledge");

    builder.add_adapter(&adapter_path, "lora-1", "lora")?;
    println!("  ✓ Added LoRA adapter: lora-1");

    builder.set_description("Example BKP package with base model, knowledge module, and adapter");
    println!("  ✓ Set description");

    builder.add_tag("example");
    builder.add_tag("demo");
    println!("  ✓ Added tags\n");

    builder.finalize(&bkp_output)?;
    println!("✓ BKP package created: {}\n", bkp_output.display());

    // Verify the output file
    let metadata = std::fs::metadata(&bkp_output)?;
    println!("Package size: {} bytes", metadata.len());
    println!();

    // Example 2: Load and inspect the BKP
    println!("=== Loading BKP Package ===\n");

    let mut loader = BkpLoader::new(&bkp_output)?;
    println!("✓ Opened BKP package");

    let manifest = loader.manifest()?;
    println!("\nManifest Information:");
    println!("  ID: {}", manifest.id);
    println!("  Name: {}", manifest.name);
    println!("  Version: {}", manifest.version);
    println!("  Created: {}", manifest.created_at);
    println!("  Format version: {}", manifest.format_version);

    println!("\nBase Model:");
    println!("  Name: {}", manifest.base_model.name);
    println!("  Architecture: {}", manifest.base_model.architecture);
    println!("  Quantization: {}", manifest.base_model.quantization);
    println!("  Size: {} bytes", manifest.base_model.size_bytes);
    println!("  Path: {}", manifest.base_model.path_in_package);

    println!("\nKMOD Modules:");
    for kmod in &manifest.kmod_modules {
        println!("  - {}", kmod.name);
        println!("    Domain: {}", kmod.domain);
        println!("    Size: {} bytes", kmod.size_bytes);
    }

    println!("\nAdapters:");
    for adapter in &manifest.adapters {
        println!("  - {}", adapter.name);
        println!("    Type: {}", adapter.adapter_type);
        println!("    Size: {} bytes", adapter.size_bytes);
    }

    println!("\nDescription: {}", manifest.description);
    println!("Tags: {:?}", manifest.tags);
    println!();

    // Example 3: Extract components
    println!("=== Extracting Components ===\n");

    let extract_dir = temp_dir.path().join("extracted");
    loader.extract_to(&extract_dir)?;
    println!("✓ Extracted entire package to: {}", extract_dir.display());

    // Verify extracted structure
    println!("\nExtracted structure:");
    for entry in walkdir::WalkDir::new(&extract_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Ok(rel_path) = path.strip_prefix(&extract_dir) {
                println!("  {}", rel_path.display());
            }
        }
    }
    println!();

    // Example 4: Extract individual components
    println!("=== Extracting Individual Components ===\n");

    match loader.extract_base_model() {
        Ok(path) => {
            println!("✓ Extracted base model to: {}", path.display());
            let size = std::fs::metadata(&path)?.len();
            println!("  Size: {} bytes", size);
        }
        Err(e) => println!("✗ Failed to extract base model: {}", e),
    }

    match loader.extract_kmod_module("knowledge") {
        Ok(path) => {
            println!("✓ Extracted KMOD module 'knowledge' to: {}", path.display());
            let size = std::fs::metadata(&path)?.len();
            println!("  Size: {} bytes", size);
        }
        Err(e) => println!("✗ Failed to extract KMOD: {}", e),
    }

    match loader.extract_adapter("lora-1") {
        Ok(path) => {
            println!("✓ Extracted adapter 'lora-1' to: {}", path.display());
            let size = std::fs::metadata(&path)?.len();
            println!("  Size: {} bytes", size);
        }
        Err(e) => println!("✗ Failed to extract adapter: {}", e),
    }
    println!();

    // Example 5: List contents
    println!("=== Package Contents ===\n");

    println!("KMOD Modules:");
    match loader.list_kmod_modules() {
        Ok(modules) => {
            if modules.is_empty() {
                println!("  (none)");
            } else {
                for name in modules {
                    println!("  - {}", name);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }

    println!("\nAdapters:");
    match loader.list_adapters() {
        Ok(adapters) => {
            if adapters.is_empty() {
                println!("  (none)");
            } else {
                for name in adapters {
                    println!("  - {}", name);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    println!("=== Example Complete ===");
    println!("\nThe BKP package contains:");
    println!("  - Manifest with metadata");
    println!("  - Compressed base model (GGUF)");
    println!("  - Knowledge modules (KMOD)");
    println!("  - Adapters (LoRA/QLoRA)");
    println!("\nAll content is zstd-compressed and can be signed with Ed25519.");

    Ok(())
}
