//! Bonsai Model Converter CLI
//!
//! Production CLI tool for converting between model formats:
//! - GGUF (llama.cpp quantized)
//! - safetensors (Hugging Face)
//! - .bkp (Bonsai Knowledge Package)
//! - HuggingFace Hub remote models

use model_converter::*;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::prelude::*;

#[derive(Parser)]
#[command(name = "bonsai-convert")]
#[command(about = "Convert models between formats: GGUF, safetensors, BKP, and HuggingFace", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(global = true, short, long)]
    verbose: bool,

    /// Set log level
    #[arg(global = true, long, value_name = "LEVEL", default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert between model formats
    Convert {
        /// Input model path or HuggingFace model ID
        #[arg(short, long, value_name = "PATH")]
        input: String,

        /// Output model path
        #[arg(short, long, value_name = "PATH")]
        output: String,

        /// Source format (auto-detect if not specified)
        #[arg(short, long, value_name = "FORMAT")]
        from: Option<String>,

        /// Target format
        #[arg(short, long, value_name = "FORMAT")]
        to: String,

        /// Model context length in tokens
        #[arg(long, default_value = "4096")]
        context_length: u32,

        /// Model name (for metadata)
        #[arg(long)]
        name: Option<String>,

        /// Model author
        #[arg(long)]
        author: Option<String>,

        /// License identifier
        #[arg(long)]
        license: Option<String>,

        /// Verify roundtrip conversion
        #[arg(long)]
        verify_roundtrip: bool,

        /// Skip compression for BKP output
        #[arg(long)]
        no_compression: bool,

        /// Number of parallel jobs for batch operations
        #[arg(long, default_value = "4")]
        parallel_jobs: usize,

        /// Request timeout in seconds
        #[arg(long, default_value = "300")]
        timeout_secs: u64,

        /// HuggingFace token (or set HF_TOKEN env var)
        #[arg(long)]
        hf_token: Option<String>,
    },

    /// Convert multiple models in batch
    Batch {
        /// Input directory containing models
        #[arg(short, long, value_name = "PATH")]
        input: PathBuf,

        /// Output directory for converted models
        #[arg(short, long, value_name = "PATH")]
        output: PathBuf,

        /// Source format
        #[arg(short, long, value_name = "FORMAT")]
        from: String,

        /// Target format
        #[arg(short, long, value_name = "FORMAT")]
        to: String,

        /// Number of parallel jobs
        #[arg(long, default_value = "4")]
        parallel_jobs: usize,
    },

    /// Validate a model file
    Validate {
        /// Model file to validate
        #[arg(value_name = "PATH")]
        path: PathBuf,

        /// Check signature for BKP files
        #[arg(long)]
        check_signature: bool,
    },

    /// List supported formats
    Formats,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let level = if cli.verbose {
        "debug"
    } else {
        &cli.log_level
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(level))
        .init();

    match cli.command {
        Commands::Convert {
            input,
            output,
            from,
            to,
            context_length,
            name,
            author,
            license,
            verify_roundtrip,
            no_compression,
            parallel_jobs,
            timeout_secs,
            hf_token,
        } => {
            let from_format = if let Some(fmt) = from {
                parse_format(&fmt)?
            } else {
                detect_format(&input)?
            };

            let to_format = parse_format(&to)?;

            let config = ConversionConfig {
                context_length,
                model_name: name,
                author,
                license,
                verify_roundtrip,
                compress_bkp: !no_compression,
                parallel_jobs,
                timeout_secs,
                ..Default::default()
            };

            println!(
                "Converting {} → {} ...",
                format_label(from_format),
                format_label(to_format)
            );

            let result = converters::convert(from_format, to_format, &input, &output, config).await;

            match result {
                Ok(()) => {
                    println!("✓ Conversion complete: {}", output);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Conversion failed: {}", e);
                    Err(anyhow::anyhow!("Conversion failed: {}", e))
                }
            }
        }

        Commands::Batch {
            input,
            output,
            from,
            to,
            parallel_jobs,
        } => {
            let from_format = parse_format(&from)?;
            let to_format = parse_format(&to)?;

            let config = ConversionConfig {
                parallel_jobs,
                ..Default::default()
            };

            println!(
                "Batch converting {} → {} in {}...",
                format_label(from_format),
                format_label(to_format),
                input.display()
            );

            let result = convert_batch(&input, &output, from_format, to_format, config).await;

            match result {
                Ok(batch_result) => {
                    println!(
                        "✓ Batch complete: {} successful, {} failed ({:.1}% success rate)",
                        batch_result.successful,
                        batch_result.failed,
                        batch_result.success_rate()
                    );

                    if !batch_result.errors.is_empty() {
                        eprintln!("\nErrors:");
                        for error in batch_result.errors {
                            eprintln!("  - {}", error);
                        }
                    }

                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Batch conversion failed: {}", e);
                    Err(anyhow::anyhow!("Batch conversion failed: {}", e))
                }
            }
        }

        Commands::Validate { path, check_signature } => {
            println!("Validating {}...", path.display());

            let result = validation::validate_model(&path);

            match result {
                Ok(validation) => {
                    println!("✓ Valid {} file", format_label(validation.format));
                    println!("  Size: {}", format_size(validation.file_size));
                    println!("  Hash: {}", &validation.hash[..16]);

                    if !validation.warnings.is_empty() {
                        println!("\nWarnings:");
                        for warning in validation.warnings {
                            println!("  - {}", warning);
                        }
                    }

                    Ok(())
                }
                Err(e) => {
                    eprintln!("✗ Validation failed: {}", e);
                    Err(anyhow::anyhow!("Validation failed: {}", e))
                }
            }
        }

        Commands::Formats => {
            println!("Supported formats:");
            println!("  - gguf           GGUF (llama.cpp)");
            println!("  - safetensors    safetensors (Hugging Face)");
            println!("  - bkp            BKP (Bonsai Knowledge Package)");
            println!("  - huggingface    HuggingFace Hub remote");
            println!("  - pytorch        PyTorch (.pth, .pt)");
            println!("  - onnx           ONNX");
            println!("\nSupported conversions:");
            println!("  - gguf ↔ bkp");
            println!("  - gguf ↔ safetensors");
            println!("  - bkp ↔ safetensors");
            println!("  - huggingface → bkp");
            println!("  - bkp → huggingface");
            println!("  - Batch operations support all above conversions");
            Ok(())
        }
    }
}

fn parse_format(format: &str) -> anyhow::Result<ModelFormat> {
    match format.to_lowercase().as_str() {
        "gguf" => Ok(ModelFormat::Gguf),
        "safetensors" | "st" => Ok(ModelFormat::Safetensors),
        "bkp" => Ok(ModelFormat::Bkp),
        "huggingface" | "hf" => Ok(ModelFormat::HuggingFace),
        "pytorch" | "pt" | "pth" => Ok(ModelFormat::PyTorch),
        "onnx" => Ok(ModelFormat::Onnx),
        _ => Err(anyhow::anyhow!(
            "Unknown format: {}. Run 'bonsai-convert formats' to see supported formats",
            format
        )),
    }
}

fn detect_format(path: &str) -> anyhow::Result<ModelFormat> {
    use std::path::Path;

    let path_obj = Path::new(path);

    // Check if it's a HF model ID
    if !path_obj.exists() && path.contains('/') && !path.contains('\\') {
        return Ok(ModelFormat::HuggingFace);
    }

    format::detect_format(path_obj).map_err(|e| anyhow::anyhow!("{}", e))
}

fn format_label(format: ModelFormat) -> &'static str {
    match format {
        ModelFormat::Gguf => "GGUF",
        ModelFormat::Safetensors => "safetensors",
        ModelFormat::Bkp => "BKP",
        ModelFormat::HuggingFace => "HuggingFace",
        ModelFormat::PyTorch => "PyTorch",
        ModelFormat::Onnx => "ONNX",
        ModelFormat::Checkpoint => "Checkpoint",
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}
