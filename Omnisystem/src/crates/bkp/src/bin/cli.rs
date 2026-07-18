//! CLI demo: builds a small .bkp package from a temp base model file and
//! reads it back, printing the manifest.

use bkp::{BkpBuilder, BkpLoader};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::TempDir::new()?;
    let output_path = temp_dir.path().join("demo.bkp");

    let model_path = temp_dir.path().join("model.gguf");
    {
        let mut file = std::fs::File::create(&model_path)?;
        file.write_all(b"gguf-demo-data")?;
    }

    let mut builder = BkpBuilder::new("demo-model", "0.1.0")?;
    builder.add_base_model(&model_path)?;
    builder.set_description("Demo BKP package built by the CLI");
    builder.add_tag("demo");
    builder.finalize(&output_path)?;

    println!("Created BKP package: {}", output_path.display());

    let mut loader = BkpLoader::new(&output_path)?;
    let manifest = loader.manifest()?;

    println!("Manifest name: {}", manifest.name);
    println!("Manifest version: {}", manifest.version);
    println!("Base model: {} ({} bytes)", manifest.base_model.name, manifest.base_model.size_bytes);
    println!("Description: {}", manifest.description);
    println!("Tags: {:?}", manifest.tags);

    Ok(())
}
