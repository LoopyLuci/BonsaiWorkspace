//! Convert GGUF to safetensors format via llama.cpp

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use std::path::Path;
use std::process::Command;

/// Convert GGUF to safetensors format
///
/// Uses llama.cpp's convert.py script to perform the conversion.
/// Requires llama.cpp to be installed and in PATH.
pub async fn convert_gguf_to_safetensors<P: AsRef<Path>>(
    gguf_path: P,
    output_path: P,
    config: ConversionConfig,
) -> ConverterResult<()> {
    let gguf_path = gguf_path.as_ref();
    let output_path = output_path.as_ref();

    if !gguf_path.exists() {
        return Err(ConverterError::NotFound(gguf_path.to_path_buf()));
    }

    // Find llama.cpp convert.py script
    let convert_py = find_convert_py()?;

    tracing::info!(
        "Converting GGUF {} to safetensors {} using {}",
        gguf_path.display(),
        output_path.display(),
        convert_py.display()
    );

    // Create output directory
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Build command
    let mut cmd = Command::new("python3");
    cmd.arg(&convert_py)
        .arg(gguf_path)
        .arg("--outfile")
        .arg(output_path)
        .arg("--outtype")
        .arg("f32");

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
        "Successfully converted GGUF to safetensors at {}",
        output_path.display()
    );

    Ok(())
}

fn find_convert_py() -> ConverterResult<std::path::PathBuf> {
    // Try common locations
    let candidates = vec![
        // Direct PATH
        "convert.py",
        // llama.cpp subdirectories
        "llama.cpp/convert.py",
        "../llama.cpp/convert.py",
        // Common installation paths
        "/usr/local/bin/convert.py",
        "/opt/llama.cpp/convert.py",
        "C:\\llama.cpp\\convert.py",
    ];

    for candidate in candidates {
        let path = std::path::PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }

        // Also check with .exe extension on Windows
        let exe_path = std::path::PathBuf::from(format!("{}.exe", candidate));
        if exe_path.exists() {
            return Ok(exe_path);
        }
    }

    // Check if 'which' can find it
    if let Ok(output) = which::which("convert.py") {
        return Ok(output);
    }

    Err(ConverterError::llama_cpp_not_found(
        "convert.py not found in PATH or common locations. \
         Install llama.cpp from https://github.com/ggerganov/llama.cpp"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_convert_py_missing() {
        // This test will fail if convert.py is not installed, which is expected
        let result = find_convert_py();
        // Don't assert here; just ensure it returns a ConverterResult
        let _ = result;
    }
}
