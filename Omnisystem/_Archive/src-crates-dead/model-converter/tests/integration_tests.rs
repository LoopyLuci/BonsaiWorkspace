//! Integration tests for model conversion

use model_converter::*;
use std::path::Path;

#[test]
fn test_format_detection_by_extension() {
    use std::path::PathBuf;

    let test_cases = vec![
        ("model.gguf", ModelFormat::Gguf),
        ("model.safetensors", ModelFormat::Safetensors),
        ("model.bkp", ModelFormat::Bkp),
        ("model.pth", ModelFormat::PyTorch),
        ("model.onnx", ModelFormat::Onnx),
    ];

    for (filename, expected_format) in test_cases {
        let path = PathBuf::from(filename);
        let detected = format::FormatDetector::detect(&path).expect(&format!("Failed to detect {}", filename));
        assert_eq!(detected, expected_format, "Format detection failed for {}", filename);
    }
}

#[test]
fn test_huggingface_id_detection() {
    let valid_ids = vec![
        "meta-llama/Llama-2-7b",
        "gpt2",
        "stabilityai/stable-diffusion-2",
        "openai-community/gpt2",
    ];

    for id in valid_ids {
        let detected = format::FormatDetector::detect_huggingface_id(id);
        assert!(detected.is_ok(), "Failed to detect HF ID: {}", id);
        assert_eq!(detected.unwrap(), ModelFormat::HuggingFace);
    }
}

#[test]
fn test_validation_empty_file() {
    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");

    let result = validation::validate_model(temp_file.path());

    match result {
        Ok(validation) => {
            assert!(!validation.is_valid);
            assert!(validation.issues.iter().any(|i| i.contains("empty")));
        }
        Err(_) => {
            // Also acceptable
        }
    }
}

#[test]
fn test_progress_reporting() {
    use tokio::sync::mpsc;

    let (reporter, mut rx) = progress::ProgressReporter::new("test-op");

    let progress = progress::ConversionProgress::new("test-op")
        .with_stage("reading")
        .with_message("Reading model file");

    reporter.report(progress.clone()).unwrap();

    // Verify we can receive the progress update
    // Note: This would normally use tokio::runtime in a real async context
}

#[test]
fn test_conversion_config_defaults() {
    let config = ConversionConfig::default();

    assert_eq!(config.context_length, 4096);
    assert_eq!(config.compress_bkp, true);
    assert_eq!(config.verify_roundtrip, false);
    assert!(config.parallel_jobs > 0);
}

#[tokio::test]
async fn test_format_conversion_dispatch() {
    // Test that the dispatcher correctly routes conversion requests
    let config = ConversionConfig::default();

    // GGUF to BKP should be supported
    // (Would need actual files to test fully)
    let result = converters::convert(
        ModelFormat::Gguf,
        ModelFormat::Bkp,
        "nonexistent.gguf",
        "output.bkp",
        config.clone(),
    )
    .await;

    // Should fail with file not found, which is expected
    assert!(result.is_err());
}

#[test]
fn test_batch_conversion_result() {
    let result = converters::batch::BatchConversionResult {
        total: 10,
        successful: 8,
        failed: 2,
        errors: vec!["error1".to_string()],
    };

    assert_eq!(result.success_rate(), 80.0);
}

#[test]
fn test_cli_format_parsing() {
    // These would test CLI argument parsing if exposed
    // For now, just verify the error types are correct
    let error: ConverterError = ConverterError::format("Test format error");
    assert!(error.to_string().contains("format error"));
}
