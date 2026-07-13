//! Batch conversion operations

use crate::error::{ConverterError, ConverterResult};
use crate::format::ModelFormat;
use crate::ConversionConfig;
use std::path::{Path, PathBuf};
use tokio::task::JoinSet;

/// Convert multiple models in batch
///
/// Converts all models of a given format in input_dir to another format,
/// placing results in output_dir. Respects parallel_jobs from ConversionConfig.
pub async fn convert_batch(
    input_dir: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    from_format: ModelFormat,
    to_format: ModelFormat,
    config: ConversionConfig,
) -> ConverterResult<BatchConversionResult> {
    let input_dir = input_dir.as_ref();
    let output_dir = output_dir.as_ref();

    if !input_dir.exists() {
        return Err(ConverterError::NotFound(input_dir.to_path_buf()));
    }

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    tracing::info!(
        "Starting batch conversion: {} → {} files",
        from_format, to_format
    );

    // Find all matching files
    let files = find_matching_files(input_dir, from_format)?;

    if files.is_empty() {
        return Ok(BatchConversionResult {
            total: 0,
            successful: 0,
            failed: 0,
            errors: Vec::new(),
        });
    }

    tracing::info!("Found {} files to convert", files.len());

    let mut tasks = JoinSet::new();
    let mut file_iter = files.into_iter();

    // Spawn up to parallel_jobs tasks
    for _ in 0..config.parallel_jobs {
        if let Some(file_path) = file_iter.next() {
            let output_dir = output_dir.to_path_buf();
            let config = config.clone();
            tasks.spawn(async move {
                convert_single_batch_file(&file_path, &output_dir, from_format, to_format, &config).await
            });
        }
    }

    let mut result = BatchConversionResult {
        total: files.len(),
        successful: 0,
        failed: 0,
        errors: Vec::new(),
    };

    // Process results and spawn new tasks
    while let Some(task_result) = tasks.join_next().await {
        match task_result {
            Ok(Ok(())) => {
                result.successful += 1;
            }
            Ok(Err(e)) => {
                result.failed += 1;
                result.errors.push(e.to_string());
            }
            Err(e) => {
                result.failed += 1;
                result.errors.push(format!("Task panicked: {}", e));
            }
        }

        // Spawn next task if available
        if let Some(file_path) = file_iter.next() {
            let output_dir = output_dir.to_path_buf();
            let config = config.clone();
            tasks.spawn(async move {
                convert_single_batch_file(&file_path, &output_dir, from_format, to_format, &config).await
            });
        }
    }

    tracing::info!(
        "Batch conversion complete: {} successful, {} failed",
        result.successful,
        result.failed
    );

    Ok(result)
}

async fn convert_single_batch_file(
    input_file: &Path,
    output_dir: &Path,
    from_format: ModelFormat,
    to_format: ModelFormat,
    config: &ConversionConfig,
) -> ConverterResult<()> {
    let filename = input_file
        .file_stem()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ConverterError::format("Invalid filename"))?;

    let output_extension = match to_format {
        ModelFormat::Gguf => "gguf",
        ModelFormat::Safetensors => "safetensors",
        ModelFormat::Bkp => "bkp",
        ModelFormat::PyTorch => "pt",
        ModelFormat::Onnx => "onnx",
        _ => "bin",
    };

    let output_path = output_dir.join(format!("{}.{}", filename, output_extension));

    tracing::info!(
        "Converting: {} → {}",
        input_file.display(),
        output_path.display()
    );

    crate::converters::convert(
        from_format,
        to_format,
        input_file,
        &output_path,
        config.clone(),
    )
    .await
}

fn find_matching_files(dir: &Path, format: ModelFormat) -> ConverterResult<Vec<PathBuf>> {
    let extensions = match format {
        ModelFormat::Gguf => vec!["gguf"],
        ModelFormat::Safetensors => vec!["safetensors"],
        ModelFormat::Bkp => vec!["bkp"],
        ModelFormat::PyTorch => vec!["pth", "pt"],
        ModelFormat::Onnx => vec!["onnx"],
        ModelFormat::Checkpoint => vec![], // Checkpoints are directories
        ModelFormat::HuggingFace => vec![], // HF models are remote
    };

    let mut files = Vec::new();

    if extensions.is_empty() {
        return Ok(files);
    }

    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext
                    .to_str()
                    .unwrap_or("")
                    .to_lowercase();
                if extensions.contains(&ext_str.as_str()) {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

/// Result summary for batch conversion operations
#[derive(Debug, Clone)]
pub struct BatchConversionResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

impl BatchConversionResult {
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.successful as f64 / self.total as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_matching_files() {
        let temp_dir = tempfile::TempDir::new().unwrap();

        // Create test files
        let gguf_path = temp_dir.path().join("model.gguf");
        std::fs::write(&gguf_path, b"test").unwrap();

        let st_path = temp_dir.path().join("model.safetensors");
        std::fs::write(&st_path, b"test").unwrap();

        let files = find_matching_files(temp_dir.path(), ModelFormat::Gguf).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("model.gguf"));
    }

    #[test]
    fn test_batch_result_success_rate() {
        let result = BatchConversionResult {
            total: 10,
            successful: 8,
            failed: 2,
            errors: vec![],
        };

        assert_eq!(result.success_rate(), 80.0);
    }
}
