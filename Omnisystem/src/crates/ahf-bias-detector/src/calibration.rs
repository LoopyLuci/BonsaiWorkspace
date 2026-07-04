//! Confidence calibration validation
//!
//! Validates that confidence scores are well-calibrated against actual correctness.
//! A well-calibrated model has:
//! - High confidence → High probability of being correct
//! - Low confidence → High probability of being incorrect
//!
//! Key metric: Expected Calibration Error (ECE)
//! - ECE < 0.05: Excellent calibration
//! - ECE 0.05-0.10: Good calibration
//! - ECE 0.10-0.15: Acceptable calibration
//! - ECE > 0.15: Poor calibration (requires recalibration)

use crate::error::BiasDetectorError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Calibration metrics for confidence scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationMetrics {
    /// Expected Calibration Error (0.0-1.0)
    pub ece: f64,
    /// Maximum Calibration Error
    pub mce: f64,
    /// Number of samples
    pub num_samples: usize,
    /// Number of bins used for ECE computation
    pub num_bins: usize,
    /// Reliability diagram data (bin midpoint, accuracy, confidence)
    pub reliability_data: Vec<(f64, f64, f64)>,
    /// Assessment of calibration quality
    pub quality_assessment: CalibrationQuality,
}

/// Calibration quality assessment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CalibrationQuality {
    /// ECE < 0.05: Excellent
    Excellent,
    /// ECE 0.05-0.10: Good
    Good,
    /// ECE 0.10-0.15: Acceptable
    Acceptable,
    /// ECE > 0.15: Poor
    Poor,
}

impl CalibrationQuality {
    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            CalibrationQuality::Excellent => "Excellent: ECE < 0.05",
            CalibrationQuality::Good => "Good: ECE 0.05-0.10",
            CalibrationQuality::Acceptable => "Acceptable: ECE 0.10-0.15",
            CalibrationQuality::Poor => "Poor: ECE > 0.15",
        }
    }
}

/// Validation sample: predicted confidence + ground truth correctness
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CalibrationSample {
    /// Predicted confidence (0.0-1.0)
    pub confidence: f64,
    /// Ground truth: was the model correct?
    pub is_correct: bool,
}

impl CalibrationSample {
    /// Create a new calibration sample
    pub fn new(confidence: f64, is_correct: bool) -> Self {
        CalibrationSample {
            confidence: confidence.clamp(0.0, 1.0),
            is_correct,
        }
    }
}

/// Calibration validator for confidence scores
pub struct CalibrationValidator {
    /// Number of bins for ECE computation (default: 10)
    num_bins: usize,
}

impl CalibrationValidator {
    /// Create a new calibration validator
    pub fn new() -> Self {
        CalibrationValidator { num_bins: 10 }
    }

    /// Set number of bins
    pub fn with_num_bins(mut self, num_bins: usize) -> Self {
        self.num_bins = num_bins.max(1);
        self
    }

    /// Get number of bins
    pub fn num_bins(&self) -> usize {
        self.num_bins
    }

    /// Compute Expected Calibration Error (ECE)
    ///
    /// ECE bins the confidence scores and compares average confidence to accuracy
    /// in each bin. The weighted average difference is the ECE.
    pub fn compute_ece(&self, samples: &[CalibrationSample]) -> Result<f64, BiasDetectorError> {
        if samples.is_empty() {
            return Err(BiasDetectorError::calibration_validation_failed(
                "No samples provided",
            ));
        }

        let mut bins: HashMap<usize, Vec<bool>> = HashMap::new();

        // Assign samples to bins
        for sample in samples {
            let bin_idx = ((sample.confidence * self.num_bins as f64).floor() as usize)
                .min(self.num_bins - 1);
            bins.entry(bin_idx)
                .or_insert_with(Vec::new)
                .push(sample.is_correct);
        }

        // Compute ECE
        let mut ece = 0.0;
        for (bin_idx, correct_flags) in bins.iter() {
            if correct_flags.is_empty() {
                continue;
            }

            let bin_confidence = (*bin_idx as f64 + 0.5) / self.num_bins as f64;
            let accuracy = correct_flags.iter().filter(|&&x| x).count() as f64 / correct_flags.len() as f64;
            let bin_weight = correct_flags.len() as f64 / samples.len() as f64;

            ece += bin_weight * (bin_confidence - accuracy).abs();
        }

        Ok(ece)
    }

    /// Compute Maximum Calibration Error (MCE)
    pub fn compute_mce(&self, samples: &[CalibrationSample]) -> Result<f64, BiasDetectorError> {
        if samples.is_empty() {
            return Err(BiasDetectorError::calibration_validation_failed(
                "No samples provided",
            ));
        }

        let mut bins: HashMap<usize, Vec<bool>> = HashMap::new();

        // Assign samples to bins
        for sample in samples {
            let bin_idx = ((sample.confidence * self.num_bins as f64).floor() as usize)
                .min(self.num_bins - 1);
            bins.entry(bin_idx)
                .or_insert_with(Vec::new)
                .push(sample.is_correct);
        }

        // Compute MCE
        let mut mce: f64 = 0.0;
        for (bin_idx, correct_flags) in bins.iter() {
            if correct_flags.is_empty() {
                continue;
            }

            let bin_confidence = (*bin_idx as f64 + 0.5) / self.num_bins as f64;
            let accuracy = correct_flags.iter().filter(|&&x| x).count() as f64 / correct_flags.len() as f64;
            let error = (bin_confidence - accuracy).abs();

            if error > mce {
                mce = error;
            }
        }

        Ok(mce)
    }

    /// Compute reliability diagram data
    pub fn compute_reliability_data(
        &self,
        samples: &[CalibrationSample],
    ) -> Result<Vec<(f64, f64, f64)>, BiasDetectorError> {
        if samples.is_empty() {
            return Err(BiasDetectorError::calibration_validation_failed(
                "No samples provided",
            ));
        }

        let mut bins: HashMap<usize, Vec<bool>> = HashMap::new();

        // Assign samples to bins
        for sample in samples {
            let bin_idx = ((sample.confidence * self.num_bins as f64).floor() as usize)
                .min(self.num_bins - 1);
            bins.entry(bin_idx)
                .or_insert_with(Vec::new)
                .push(sample.is_correct);
        }

        // Compute reliability data
        let mut data = Vec::new();
        for bin_idx in 0..self.num_bins {
            if let Some(correct_flags) = bins.get(&bin_idx) {
                if correct_flags.is_empty() {
                    continue;
                }

                let bin_confidence = (bin_idx as f64 + 0.5) / self.num_bins as f64;
                let accuracy = correct_flags.iter().filter(|&&x| x).count() as f64 / correct_flags.len() as f64;

                data.push((bin_confidence, accuracy, bin_confidence));
            }
        }

        Ok(data)
    }

    /// Assess calibration quality based on ECE
    fn assess_quality(&self, ece: f64) -> CalibrationQuality {
        if ece < 0.05 {
            CalibrationQuality::Excellent
        } else if ece < 0.10 {
            CalibrationQuality::Good
        } else if ece < 0.15 {
            CalibrationQuality::Acceptable
        } else {
            CalibrationQuality::Poor
        }
    }

    /// Validate calibration on a set of samples
    pub fn validate(&self, samples: &[CalibrationSample]) -> Result<CalibrationMetrics, BiasDetectorError> {
        if samples.is_empty() {
            return Err(BiasDetectorError::calibration_validation_failed(
                "No samples provided",
            ));
        }

        let ece = self.compute_ece(samples)?;
        let mce = self.compute_mce(samples)?;
        let reliability_data = self.compute_reliability_data(samples)?;
        let quality_assessment = self.assess_quality(ece);

        Ok(CalibrationMetrics {
            ece,
            mce,
            num_samples: samples.len(),
            num_bins: self.num_bins,
            reliability_data,
            quality_assessment,
        })
    }

    /// Flag miscalibrated models (ECE > 0.15)
    pub fn is_miscalibrated(&self, metrics: &CalibrationMetrics) -> bool {
        metrics.quality_assessment == CalibrationQuality::Poor
    }
}

impl Default for CalibrationValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_sample_creation() {
        let sample = CalibrationSample::new(0.9, true);
        assert_eq!(sample.confidence, 0.9);
        assert!(sample.is_correct);
    }

    #[test]
    fn test_calibration_sample_clamping() {
        let sample = CalibrationSample::new(1.5, true);
        assert_eq!(sample.confidence, 1.0);

        let sample = CalibrationSample::new(-0.5, false);
        assert_eq!(sample.confidence, 0.0);
    }

    #[test]
    fn test_calibration_quality_descriptions() {
        assert!(CalibrationQuality::Excellent.description().contains("0.05"));
        assert!(CalibrationQuality::Good.description().contains("0.05"));
        assert!(CalibrationQuality::Acceptable.description().contains("0.10"));
        assert!(CalibrationQuality::Poor.description().contains("0.15"));
    }

    #[test]
    fn test_calibration_validator_creation() {
        let validator = CalibrationValidator::new();
        assert_eq!(validator.num_bins(), 10);
    }

    #[test]
    fn test_calibration_validator_with_bins() {
        let validator = CalibrationValidator::new().with_num_bins(5);
        assert_eq!(validator.num_bins(), 5);
    }

    #[test]
    fn test_ece_empty_samples() {
        let validator = CalibrationValidator::new();
        let result = validator.compute_ece(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_ece_perfect_calibration() {
        let validator = CalibrationValidator::new();
        // Perfect calibration: 90% confidence, 90% correct
        let samples: Vec<CalibrationSample> = (0..10)
            .map(|_| CalibrationSample::new(0.9, true))
            .collect();
        let ece = validator.compute_ece(&samples).expect("ECE computation failed");
        assert!(ece < 0.1); // Should be well-calibrated
    }

    #[test]
    fn test_ece_poor_calibration() {
        let validator = CalibrationValidator::new();
        // Poor calibration: 90% confidence, 0% correct
        let samples: Vec<CalibrationSample> = (0..10)
            .map(|_| CalibrationSample::new(0.9, false))
            .collect();
        let ece = validator.compute_ece(&samples).expect("ECE computation failed");
        assert!(ece > 0.5); // Should be poorly calibrated
    }

    #[test]
    fn test_mce_empty_samples() {
        let validator = CalibrationValidator::new();
        let result = validator.compute_mce(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mce_computation() {
        let validator = CalibrationValidator::new();
        let samples = vec![
            CalibrationSample::new(0.9, true),
            CalibrationSample::new(0.9, true),
            CalibrationSample::new(0.1, false),
            CalibrationSample::new(0.1, false),
        ];
        let mce = validator.compute_mce(&samples).expect("MCE computation failed");
        assert!(mce >= 0.0 && mce <= 1.0);
    }

    #[test]
    fn test_reliability_data_empty() {
        let validator = CalibrationValidator::new();
        let result = validator.compute_reliability_data(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_reliability_data_computation() {
        let validator = CalibrationValidator::new();
        let samples = vec![
            CalibrationSample::new(0.9, true),
            CalibrationSample::new(0.9, true),
            CalibrationSample::new(0.1, false),
            CalibrationSample::new(0.1, false),
        ];
        let data = validator
            .compute_reliability_data(&samples)
            .expect("Reliability data computation failed");
        assert!(!data.is_empty());
        // All values should be between 0 and 1
        for (conf, acc, _) in data {
            assert!(conf >= 0.0 && conf <= 1.0);
            assert!(acc >= 0.0 && acc <= 1.0);
        }
    }

    #[test]
    fn test_assess_quality_excellent() {
        let validator = CalibrationValidator::new();
        assert_eq!(validator.assess_quality(0.03), CalibrationQuality::Excellent);
    }

    #[test]
    fn test_assess_quality_good() {
        let validator = CalibrationValidator::new();
        assert_eq!(validator.assess_quality(0.07), CalibrationQuality::Good);
    }

    #[test]
    fn test_assess_quality_acceptable() {
        let validator = CalibrationValidator::new();
        assert_eq!(validator.assess_quality(0.12), CalibrationQuality::Acceptable);
    }

    #[test]
    fn test_assess_quality_poor() {
        let validator = CalibrationValidator::new();
        assert_eq!(validator.assess_quality(0.2), CalibrationQuality::Poor);
    }

    #[test]
    fn test_validate_perfect_calibration() {
        let validator = CalibrationValidator::new();
        let samples: Vec<CalibrationSample> = (0..20)
            .map(|i| CalibrationSample::new(if i < 10 { 0.9 } else { 0.1 }, i < 10))
            .collect();
        let metrics = validator.validate(&samples).expect("Validation failed");
        assert!(metrics.ece < 0.2);
    }

    #[test]
    fn test_validate_with_reliability_data() {
        let validator = CalibrationValidator::new();
        let samples = vec![
            CalibrationSample::new(0.9, true),
            CalibrationSample::new(0.9, true),
        ];
        let metrics = validator.validate(&samples).expect("Validation failed");
        assert!(!metrics.reliability_data.is_empty());
    }

    #[test]
    fn test_is_miscalibrated() {
        let validator = CalibrationValidator::new();

        let samples: Vec<CalibrationSample> = (0..10)
            .map(|_| CalibrationSample::new(0.9, false))
            .collect();
        let metrics = validator.validate(&samples).expect("Validation failed");
        assert!(validator.is_miscalibrated(&metrics));
    }

    #[test]
    fn test_is_well_calibrated() {
        let validator = CalibrationValidator::new();

        let samples: Vec<CalibrationSample> = (0..20)
            .map(|i| CalibrationSample::new(if i < 10 { 0.9 } else { 0.1 }, i < 10))
            .collect();
        let metrics = validator.validate(&samples).expect("Validation failed");
        assert!(!validator.is_miscalibrated(&metrics));
    }
}
