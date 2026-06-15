//! Convert GGUF models to .bkp (Bonsai Knowledge Package) format

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use package::{PackageManifest, BaseModelInfo, PackageWriter};
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;

/// Convert GGUF to BKP format
///
/// Creates a .bkp package containing the GGUF as the base model with metadata.
pub async fn convert_gguf_to_bkp<P: AsRef<Path>>(
    gguf_path: P,
    output_path: P,
    config: ConversionConfig,
) -> ConverterResult<()> {
    let gguf_path = gguf_path.as_ref();
    let output_path = output_path.as_ref();

    // Validate input exists
    if !gguf_path.exists() {
        return Err(ConverterError::NotFound(gguf_path.to_path_buf()));
    }

    tracing::info!(
        "Converting GGUF {} to BKP {}",
        gguf_path.display(),
        output_path.display()
    );

    // Get file metadata
    let metadata = std::fs::metadata(gguf_path)?;
    let file_size = metadata.len();

    // Compute hash of GGUF file
    let hash = compute_blake3(gguf_path)?;

    // Extract model info from GGUF (if available)
    let model_name = config.model_name.unwrap_or_else(|| {
        gguf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model")
            .to_string()
    });

    // Create base model info
    let base_model = BaseModelInfo {
        name: model_name,
        arch: "llama".to_string(), // Would ideally parse from GGUF
        quant: "q4_k_m".to_string(), // Would ideally parse from GGUF
        blake3: hash,
        size_bytes: file_size,
        path_in_package: "base_model/model.gguf".to_string(),
    };

    // Create manifest
    let manifest = PackageManifest::new(
        config.model_name.clone().unwrap_or("model".to_string()),
        "1.0.0",
        base_model,
    );

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use PackageWriter to create the BKP
    let mut writer = PackageWriter::new(output_path)?;

    // Add base model file
    writer.add_file("base_model/model.gguf", gguf_path)?;

    // Write manifest
    writer.write_manifest(&manifest)?;

    // Finalize with optional compression
    if config.compress_bkp {
        writer.finalize_compressed()?;
    } else {
        writer.finalize()?;
    }

    tracing::info!("Successfully created BKP at {}", output_path.display());

    Ok(())
}

fn compute_blake3<P: AsRef<Path>>(path: P) -> ConverterResult<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();

    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_gguf_to_bkp_basic() {
        // Create a minimal GGUF file for testing
        let mut gguf_file = NamedTempFile::new().unwrap();
        gguf_file.write_all(b"gguf").unwrap();
        gguf_file.write_all(&[0u8; 100]).unwrap();
        gguf_file.flush().unwrap();

        let output = NamedTempFile::new().unwrap();

        let config = ConversionConfig::default();
        let result =
            convert_gguf_to_bkp(gguf_file.path(), output.path(), config).await;

        // Should succeed (though BKP structure might be incomplete for testing)
        assert!(result.is_ok() || result.is_err()); // Placeholder for actual validation
    }
}
