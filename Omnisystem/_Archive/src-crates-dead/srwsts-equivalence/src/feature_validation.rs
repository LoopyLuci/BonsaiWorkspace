//! Architecture-specific feature validation
//!
//! Validates SIMD, cryptographic instructions, and other architecture-specific features.

use crate::{ArchitectureTarget, ArchitectureFeatures, EquivalenceResult, EquivalenceError};
use serde::{Deserialize, Serialize};

/// SIMD test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SIMDTestResult {
    /// Architecture tested
    pub architecture: String,
    /// Vector operation
    pub operation: String,
    /// Input vectors
    pub inputs: Vec<Vec<u32>>,
    /// Output vector
    pub output: Vec<u32>,
    /// Expected output
    pub expected: Vec<u32>,
    /// Whether outputs match
    pub matches: bool,
    /// Performance vs scalar (multiple)
    pub speedup: f64,
}

impl SIMDTestResult {
    /// Create a new SIMD test result
    pub fn new(
        architecture: String,
        operation: String,
        inputs: Vec<Vec<u32>>,
        output: Vec<u32>,
        expected: Vec<u32>,
        speedup: f64,
    ) -> Self {
        let matches = output == expected;

        Self {
            architecture,
            operation,
            inputs,
            output,
            expected,
            matches,
            speedup,
        }
    }

    /// Check if output is correct
    pub fn is_correct(&self) -> bool {
        self.matches
    }

    /// Check if speedup is acceptable (>1.5x for 4-wide)
    pub fn speedup_acceptable(&self) -> bool {
        self.speedup >= 1.5
    }
}

/// AES-NI test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AESNITest {
    /// Architecture
    pub architecture: String,
    /// Plaintext
    pub plaintext: Vec<u8>,
    /// Key
    pub key: Vec<u8>,
    /// Ciphertext
    pub ciphertext: Vec<u8>,
    /// Expected ciphertext
    pub expected_ciphertext: Vec<u8>,
    /// Whether encryption is correct
    pub correct: bool,
}

impl AESNITest {
    /// Create a new AES-NI test
    pub fn new(
        architecture: String,
        plaintext: Vec<u8>,
        key: Vec<u8>,
        ciphertext: Vec<u8>,
        expected_ciphertext: Vec<u8>,
    ) -> Self {
        let correct = ciphertext == expected_ciphertext;

        Self {
            architecture,
            plaintext,
            key,
            ciphertext,
            expected_ciphertext,
            correct,
        }
    }
}

/// CLMUL test (Carry-less multiplication)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLMULTest {
    /// Architecture
    pub architecture: String,
    /// Left operand
    pub left: u64,
    /// Right operand
    pub right: u64,
    /// Result
    pub result: u128,
    /// Expected result
    pub expected_result: u128,
    /// Correct
    pub correct: bool,
}

impl CLMULTest {
    /// Create a new CLMUL test
    pub fn new(architecture: String, left: u64, right: u64, result: u128, expected: u128) -> Self {
        Self {
            architecture,
            left,
            right,
            result,
            expected_result: expected,
            correct: result == expected,
        }
    }
}

/// RDRAND / RDSEED test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomGeneratorTest {
    /// Architecture
    pub architecture: String,
    /// Random value 1
    pub value1: u64,
    /// Random value 2
    pub value2: u64,
    /// Random value 3
    pub value3: u64,
    /// Whether values are different (statistically should be)
    pub values_different: bool,
    /// Entropy estimate (bits)
    pub entropy_bits: f64,
}

impl RandomGeneratorTest {
    /// Create a new random generator test
    pub fn new(architecture: String, value1: u64, value2: u64, value3: u64) -> Self {
        let values_different = value1 != value2 && value2 != value3 && value1 != value3;

        // Simple entropy estimate: count bits that differ across samples
        let xor1_2 = value1 ^ value2;
        let xor2_3 = value2 ^ value3;
        let xor_all = xor1_2 | xor2_3;
        let entropy_bits = xor_all.count_ones() as f64;

        Self {
            architecture,
            value1,
            value2,
            value3,
            values_different,
            entropy_bits,
        }
    }

    /// Check if entropy is high enough (>50 bits)
    pub fn entropy_sufficient(&self) -> bool {
        self.entropy_bits >= 50.0
    }
}

/// NEON intrinsics test (ARM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NEONTest {
    /// Architecture (must be ARM)
    pub architecture: String,
    /// Input vector 1
    pub input1: Vec<i32>,
    /// Input vector 2
    pub input2: Vec<i32>,
    /// Operation (e.g., "add", "multiply")
    pub operation: String,
    /// Output vector
    pub output: Vec<i32>,
    /// Expected output
    pub expected: Vec<i32>,
    /// Correct
    pub correct: bool,
}

impl NEONTest {
    /// Create a new NEON test
    pub fn new(
        architecture: String,
        input1: Vec<i32>,
        input2: Vec<i32>,
        operation: String,
        output: Vec<i32>,
        expected: Vec<i32>,
    ) -> Self {
        let correct = output == expected;

        Self {
            architecture,
            input1,
            input2,
            operation,
            output,
            expected,
            correct,
        }
    }
}

/// RV Vector Extension test (RISC-V)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RVVectorTest {
    /// Architecture
    pub architecture: String,
    /// Vector length in bits
    pub vlen_bits: usize,
    /// Input vector
    pub input: Vec<u32>,
    /// Output vector
    pub output: Vec<u32>,
    /// Expected output
    pub expected: Vec<u32>,
    /// Correct
    pub correct: bool,
}

impl RVVectorTest {
    /// Create a new RV Vector test
    pub fn new(
        architecture: String,
        vlen_bits: usize,
        input: Vec<u32>,
        output: Vec<u32>,
        expected: Vec<u32>,
    ) -> Self {
        let correct = output == expected;

        Self {
            architecture,
            vlen_bits,
            input,
            output,
            expected,
            correct,
        }
    }
}

/// Feature validator
pub struct FeatureValidator {
    features: ArchitectureFeatures,
}

impl FeatureValidator {
    /// Create a new feature validator for an architecture
    pub fn for_architecture(arch: &ArchitectureTarget) -> Self {
        Self {
            features: ArchitectureFeatures::for_architecture(arch),
        }
    }

    /// Validate SSE support
    pub fn validate_sse(&self) -> EquivalenceResult<()> {
        if !self.features.sse {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "SSE".to_string(),
            });
        }
        Ok(())
    }

    /// Validate AVX support
    pub fn validate_avx(&self) -> EquivalenceResult<()> {
        if !self.features.avx {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "AVX".to_string(),
            });
        }
        Ok(())
    }

    /// Validate AVX2 support
    pub fn validate_avx2(&self) -> EquivalenceResult<()> {
        if !self.features.avx2 {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "AVX2".to_string(),
            });
        }
        Ok(())
    }

    /// Validate AVX-512 support
    pub fn validate_avx512(&self) -> EquivalenceResult<()> {
        if !self.features.avx512 {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "AVX-512".to_string(),
            });
        }
        Ok(())
    }

    /// Validate NEON support
    pub fn validate_neon(&self) -> EquivalenceResult<()> {
        if !self.features.neon {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "NEON".to_string(),
            });
        }
        Ok(())
    }

    /// Validate AES-NI support
    pub fn validate_aes_ni(&self) -> EquivalenceResult<()> {
        if !self.features.aes_ni {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "AES-NI".to_string(),
            });
        }
        Ok(())
    }

    /// Validate CLMUL support
    pub fn validate_clmul(&self) -> EquivalenceResult<()> {
        if !self.features.clmul {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "CLMUL".to_string(),
            });
        }
        Ok(())
    }

    /// Validate RDRAND support
    pub fn validate_rdrand(&self) -> EquivalenceResult<()> {
        if !self.features.rdrand {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "RDRAND".to_string(),
            });
        }
        Ok(())
    }

    /// Validate RDSEED support
    pub fn validate_rdseed(&self) -> EquivalenceResult<()> {
        if !self.features.rdseed {
            return Err(EquivalenceError::FeatureNotAvailable {
                arch: "current".to_string(),
                feature: "RDSEED".to_string(),
            });
        }
        Ok(())
    }
}

/// Feature impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureImpactAnalysis {
    /// Feature name
    pub feature: String,
    /// Architectures with feature
    pub architectures_with: Vec<String>,
    /// Architectures without feature
    pub architectures_without: Vec<String>,
    /// Performance impact (with/without ratio)
    pub performance_impact: f64,
    /// Correctness maintained without feature
    pub fallback_correct: bool,
}

impl FeatureImpactAnalysis {
    /// Create a new feature impact analysis
    pub fn new(
        feature: String,
        architectures_with: Vec<String>,
        architectures_without: Vec<String>,
        performance_impact: f64,
        fallback_correct: bool,
    ) -> Self {
        Self {
            feature,
            architectures_with,
            architectures_without,
            performance_impact,
            fallback_correct,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_result() {
        let result = SIMDTestResult::new(
            "x86_64".to_string(),
            "add".to_string(),
            vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]],
            vec![6, 8, 10, 12],
            vec![6, 8, 10, 12],
            2.5,
        );

        assert!(result.is_correct());
        assert!(result.speedup_acceptable());
    }

    #[test]
    fn test_aes_ni() {
        let test = AESNITest::new(
            "x86_64".to_string(),
            vec![0; 16],
            vec![0; 16],
            vec![0x66; 16],
            vec![0x66; 16],
        );

        assert!(test.correct);
    }

    #[test]
    fn test_clmul() {
        let test = CLMULTest::new("x86_64".to_string(), 0x1, 0x1, 0x1, 0x1);
        assert!(test.correct);
    }

    #[test]
    fn test_random_generator() {
        let test = RandomGeneratorTest::new("x86_64".to_string(), 1, 2, 3);
        assert!(test.values_different);
    }

    #[test]
    fn test_neon() {
        let test = NEONTest::new(
            "armv8".to_string(),
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            "add".to_string(),
            vec![6, 8, 10, 12],
            vec![6, 8, 10, 12],
        );

        assert!(test.correct);
    }

    #[test]
    fn test_rvvector() {
        let test = RVVectorTest::new(
            "riscv64".to_string(),
            128,
            vec![1, 2, 3, 4],
            vec![1, 2, 3, 4],
            vec![1, 2, 3, 4],
        );

        assert!(test.correct);
    }
}
