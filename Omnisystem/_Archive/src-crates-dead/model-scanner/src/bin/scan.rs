use anyhow::Result;
use std::path::PathBuf;
use clap::Parser;
use model_scanner::ModelScanner;
use std::fs;

#[derive(Parser, Debug)]
#[command(name = "bonsai-scan")]
#[command(about = "Scan directory for ML models and generate inventory")]
struct Args {
    /// Path to scan for models
    #[arg(short, long, default_value = "D:\\Models\\general")]
    directory: PathBuf,

    /// Output JSON file
    #[arg(short, long, default_value = "model_inventory.json")]
    output: PathBuf,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    println!("🔍 Scanning directory: {}", args.directory.display());

    let scanner = ModelScanner::new();
    let models = scanner.scan(&args.directory)?;

    println!("📊 Found {} models", models.len());
    println!("📈 Sorting by size (smallest first)...\n");

    for (idx, model) in models.iter().enumerate() {
        let size_gb = model.size_bytes as f64 / 1_000_000_000.0;
        let params = model
            .parameter_count
            .map(|p| format!("{:.1}B", p as f64 / 1_000_000_000.0))
            .unwrap_or_else(|| "unknown".to_string());

        println!(
            "  {:3}. {} | {:6.2} GB | {} params | {} | {}",
            idx + 1,
            model.filename,
            size_gb,
            params,
            model.format.to_string(),
            model.quantization.as_deref().unwrap_or("unquantized")
        );
    }

    // Write to JSON
    let json = serde_json::to_string_pretty(&models)?;
    fs::write(&args.output, json)?;

    println!("\n✅ Inventory written to: {}", args.output.display());
    println!("   Total: {} models, {:.2} GB total size",
        models.len(),
        models.iter().map(|m| m.size_bytes).sum::<u64>() as f64 / 1_000_000_000.0
    );

    Ok(())
}
