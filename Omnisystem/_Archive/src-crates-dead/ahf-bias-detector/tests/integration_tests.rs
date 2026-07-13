//! Integration tests for ahf-bias-detector
//!
//! Tests the complete bias detection and confidence extraction pipeline.

use ahf_bias_detector::{
    BiasLevel, BiasPattern, BiasScoreAggregator, BiasScoreResult,
    CalibrationValidator, ConfidenceExtractor, ConfidenceScore,
    PatternMatcher, SelfConsistencySampler,
};
use ahf_bias_detector::calibration::CalibrationSample;
use ahf_bias_detector::confidence::ConfidenceSource;

#[test]
fn test_end_to_end_bias_detection() {
    // Create pattern matcher
    let matcher = PatternMatcher::default();

    // Test text with obvious stereotypes
    let text = "all women are emotional and men from that country are aggressive";

    // Detect violations
    let violations = matcher.match_all(text);
    assert!(!violations.is_empty(), "Should detect bias violations");

    // Create score
    let score_result = BiasScoreResult::from_patterns(violations);
    assert!(score_result.has_bias());
    assert!(score_result.score > 0.0);
}

#[test]
fn test_bias_score_aggregation() {
    let matcher = PatternMatcher::default();
    let text = "people from that region are naturally lazy";

    let violations = matcher.match_all(text);
    let score_result = BiasScoreResult::from_patterns(violations);

    // Create aggregator
    let agg = BiasScoreAggregator::new()
        .with_patterns_enabled(true)
        .with_min_severity(BiasLevel::Medium);

    assert!(agg.patterns_enabled());
    assert_eq!(agg.min_severity(), BiasLevel::Medium);
}

#[test]
fn test_confidence_extraction_from_json() {
    let extractor = ConfidenceExtractor::new();

    // Test JSON extraction
    let json = r#"{"confidence": 0.92, "text": "response"}"#;
    let result = extractor
        .extract_or_infer(json, true)
        .expect("Extraction failed");

    assert_eq!(result.score, 0.92);
    assert_eq!(result.source, ConfidenceSource::Extracted);
}

#[test]
fn test_confidence_extraction_from_text() {
    let extractor = ConfidenceExtractor::new();

    // Test text extraction
    let text = "The model has confidence: 0.87 in this prediction";
    let result = extractor
        .extract_or_infer(text, false)
        .expect("Extraction failed");

    assert_eq!(result.score, 0.87);
}

#[test]
fn test_self_consistency_sampling() {
    let extractor = ConfidenceExtractor::new();
    let _sampler = SelfConsistencySampler::new().with_num_samples(5);

    // Simulate 5 identical responses
    let samples = vec![
        ahf_bias_detector::ConfidenceSample::new("The answer is yes"),
        ahf_bias_detector::ConfidenceSample::new("The answer is yes"),
        ahf_bias_detector::ConfidenceSample::new("The answer is yes"),
        ahf_bias_detector::ConfidenceSample::new("The answer is yes"),
        ahf_bias_detector::ConfidenceSample::new("The answer is yes"),
    ];

    let result = extractor
        .process_consistency_samples(samples)
        .expect("Processing failed");

    assert!(result.score > 0.9);
    assert_eq!(result.source, ConfidenceSource::SelfConsistency);
    assert_eq!(result.num_samples, Some(5));
}

#[test]
fn test_calibration_validation() {
    let validator = CalibrationValidator::new();

    // Create well-calibrated samples
    let mut samples = Vec::new();
    // High confidence predictions that are correct
    for _ in 0..10 {
        samples.push(CalibrationSample::new(0.9, true));
    }
    // Low confidence predictions that are incorrect
    for _ in 0..10 {
        samples.push(CalibrationSample::new(0.1, false));
    }

    let metrics = validator.validate(&samples).expect("Validation failed");

    assert!(!validator.is_miscalibrated(&metrics));
    assert!(metrics.ece < 0.15);
}

#[test]
fn test_full_pipeline() {
    // 1. Detect bias in text
    let matcher = PatternMatcher::default();
    let biased_text = "all people from that country are naturally lazy";
    let violations = matcher.match_all(biased_text);
    assert!(!violations.is_empty());

    // 2. Create bias score
    let bias_score = BiasScoreResult::from_patterns(violations);
    assert!(bias_score.has_bias());

    // 3. Extract confidence
    let extractor = ConfidenceExtractor::new();
    let json_response = r#"{"response": "...", "confidence": 0.75}"#;
    let confidence = extractor
        .extract_or_infer(json_response, true)
        .expect("Confidence extraction failed");

    assert_eq!(confidence.score, 0.75);

    // 4. Validate calibration
    let validator = CalibrationValidator::new();
    let calibration_samples = vec![
        CalibrationSample::new(0.75, false), // Since we detected bias
        CalibrationSample::new(0.8, true),
        CalibrationSample::new(0.7, false),
    ];
    let metrics = validator
        .validate(&calibration_samples)
        .expect("Calibration validation failed");

    assert!(metrics.num_samples == 3);
}

#[test]
fn test_pattern_matcher_with_custom_patterns() {
    // Create custom pattern
    let custom_pattern = BiasPattern::new(
        "test-slur",
        r"(?i)\b(badword)\b",
        "Custom offensive term",
        BiasLevel::Critical,
        "custom",
    )
    .expect("Pattern creation failed")
    .with_example("Contains badword");

    let mut matcher = PatternMatcher::new().expect("Matcher creation failed");
    matcher.add_pattern(custom_pattern);

    // Test detection
    let text = "This text contains badword";
    assert!(matcher.contains_bias(text));
}

#[test]
fn test_severity_filtering() {
    let matcher = PatternMatcher::default();
    let text = "all women are emotional and people from there are savage";

    // Get all violations
    let all_violations = matcher.match_all(text);

    // Filter by severity
    let high_severity = matcher.match_with_severity(text, BiasLevel::High);

    assert!(all_violations.len() >= high_severity.len());
    for v in high_severity {
        assert!(v.severity >= BiasLevel::High);
    }
}

#[test]
fn test_neutral_text_no_bias_detected() {
    let matcher = PatternMatcher::default();
    let neutral_text = "The weather is nice today and the sky is blue.";

    let violations = matcher.match_all(neutral_text);
    assert!(violations.is_empty());
    assert!(!matcher.contains_bias(neutral_text));
}

#[test]
fn test_multiple_bias_types() {
    let matcher = PatternMatcher::default();

    // Text with multiple bias types
    let text = "all women are naturally emotional, and those people from that region are primitiv";

    let violations = matcher.match_all(text);
    assert!(!violations.is_empty());

    // Should find different categories
    let categories: std::collections::HashSet<_> =
        violations.iter().map(|v| &v.pattern.category).collect();
    assert!(categories.len() > 0);
}

#[test]
fn test_confidence_score_categories() {
    // High confidence
    let high = ConfidenceScore::new(0.95, ConfidenceSource::Extracted);
    assert!(high.is_high());
    assert!(!high.is_medium());
    assert!(!high.is_low());

    // Medium confidence
    let medium = ConfidenceScore::new(0.65, ConfidenceSource::Extracted);
    assert!(!medium.is_high());
    assert!(medium.is_medium());
    assert!(!medium.is_low());

    // Low confidence
    let low = ConfidenceScore::new(0.35, ConfidenceSource::Extracted);
    assert!(!low.is_high());
    assert!(!low.is_medium());
    assert!(low.is_low());
}

#[test]
fn test_consistency_sampling_mixed_responses() {
    let extractor = ConfidenceExtractor::new();

    // Mix of consistent and inconsistent responses
    let samples = vec![
        ahf_bias_detector::ConfidenceSample::new("Answer is yes"),
        ahf_bias_detector::ConfidenceSample::new("Answer is yes"),
        ahf_bias_detector::ConfidenceSample::new("Answer is yes"),
        ahf_bias_detector::ConfidenceSample::new("Answer is no"),
        ahf_bias_detector::ConfidenceSample::new("Answer is maybe"),
    ];

    let result = extractor
        .process_consistency_samples(samples)
        .expect("Processing failed");

    // 3 out of 5 are consistent
    assert!(result.consistency_ratio.is_some());
    assert!((result.consistency_ratio.unwrap() - 0.6).abs() < 0.01);
}
