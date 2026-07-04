//! Convert safetensors to GGUF format via llama.cpp

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use std::path::Path;
use std::process::Command;

/// Convert safetensors to GGUF format
///
/// Uses llama.cpp's convert.py script in reverse to perform the conversion.
pub async fn convert_safetensors_to_gguf<P: AsRef<Path>>(
    safetensors_path: P,
    output_path: P,
    config: ConversionConfig,
) -> ConverterResult<()> {
    let safetensors_path = safetensors_path.as_ref();
    let output_path = output_path.as_ref();

    if !safetensors_path.exists() {
        return Err(ConverterError::NotFound(safetensors_path.to_path_buf()));
    }

    // Find llama.cpp convert.py script
    let convert_py = find_convert_py()?;

    tracing::info!(
        "Converting safetensors {} to GGUF {} using {}",
        safetensors_path.display(),
        output_path.display(),
        convert_py.display()
    );

    // Create output directory
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Build command to convert safetensors to GGUF
    let mut cmd = Command::new("python3");
    cmd.arg(&convert_py)
        .arg(safetensors_path)
        .arg("--outfile")
        .arg(output_path)
        .arg("--outtype")
        .arg("q8_0");

    // Execute conversion
    let output = cmd
        .output()
        .map_err(|e| {
            ConverterError::Subprocess(format!(
                "Failed to execute convert.py: {}",
                e
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ConverterError::Subprocess(format!(
            "convert.py failed: {}",
            stderr
        )));
    }

    tracing::info!(
        "Successfully converted safetensors to GGUF at {}",
        output_path.display()
    );

    Ok(())
}

fn find_convert_py() -> ConverterResult<std::path::PathBuf> {
    // Try common locations
    let candidates = vec![
        "convert.py",
        "llama.cpp/convert.py",
        "../llama.cpp/convert.py",
        "/usr/local/bin/convert.py",
        "/opt/llama.cpp/convert.py",
        "C:\\llama.cpp\\convert.py",
    ];

    for candidate in candidates {
        let path = std::path::PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }

        let exe_path = std::path::PathBuf::from(format!("{}.exe", candidate));
        if exe_path.exists() {
            return Ok(exe_path);
        }
    }

    if let Ok(output) = which::which("convert.py") {
        return Ok(output);
    }

    Err(ConverterError::llama_cpp_not_found(
        "convert.py not found. Install llama.cpp from https://github.com/ggerganov/llama.cpp"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safetensors_to_gguf_not_found() {
        let result = convert_safetensors_to_gguf(
            "/nonexistent/file.safetensors",
            "/tmp/output.gguf",
            ConversionConfig::default(),
        )
        .await;

        assert!(result.is_err());
    }
}
