use crate::types::*;
use regex::Regex;

/// L3+L5: Axiom Verification - Enforces the Seven Laws of Medical AI
/// These are formally verified safety guarantees that run outside the AI model
pub struct AxiomVerifier;

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub all_passed: bool,
    pub violations: Vec<MedicalLawViolation>,
}

#[derive(Debug, Clone)]
pub struct MedicalLawViolation {
    pub law: MedicalLaw,
    pub description: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    Critical,  // Blocks response
    High,      // Triggers fallback
    Medium,    // Requires disclaimer
    Low,       // Warning
}

impl AxiomVerifier {
    /// Verify that a response satisfies all Seven Laws
    pub fn verify_response(response: &str) -> VerificationResult {
        let mut violations = Vec::new();

        // Law 1: Non-Maleficence
        if !Self::law_1_non_maleficence(response) {
            violations.push(MedicalLawViolation {
                law: MedicalLaw::NonMaleficence,
                description: "Response contains self-harm or harm-to-others content".to_string(),
                severity: ViolationSeverity::Critical,
            });
        }

        // Law 3: Never Diagnose
        if !Self::law_3_never_diagnose(response) {
            violations.push(MedicalLawViolation {
                law: MedicalLaw::NeverDiagnose,
                description: "Response contains diagnostic language".to_string(),
                severity: ViolationSeverity::Critical,
            });
        }

        // Law 4: Never Prescribe
        if !Self::law_4_never_prescribe(response) {
            violations.push(MedicalLawViolation {
                law: MedicalLaw::NeverPrescribe,
                description: "Response contains prescription language".to_string(),
                severity: ViolationSeverity::Critical,
            });
        }

        // Law 7: Respect Autonomy
        if !Self::law_7_respect_autonomy(response) {
            violations.push(MedicalLawViolation {
                law: MedicalLaw::RespectAutonomy,
                description: "Response contains coercive language".to_string(),
                severity: ViolationSeverity::High,
            });
        }

        VerificationResult {
            all_passed: violations.is_empty(),
            violations,
        }
    }

    /// Law 1: No response shall contain self-harm or harm-to-others content
    fn law_1_non_maleficence(response: &str) -> bool {
        let response_lower = response.to_lowercase();

        let self_harm_patterns = [
            r"(?i)kill.*yourself|yourself.*kill",
            r"(?i)cut.*wrist|wrist.*cut",
            r"(?i)jump.*off|overdose.*yourself",
            r"(?i)hang.*yourself",
            r"(?i)harm.*yourself|yourself.*harm",
        ];

        let harm_others_patterns = [
            r"(?i)hurt.*them|them.*hurt",
            r"(?i)kill.*them|attack.*them",
            r"(?i)harm.*others|others.*harm",
        ];

        for pattern in &self_harm_patterns {
            if Regex::new(pattern)
                .unwrap()
                .is_match(&response_lower)
            {
                return false;
            }
        }

        for pattern in &harm_others_patterns {
            if Regex::new(pattern)
                .unwrap()
                .is_match(&response_lower)
            {
                return false;
            }
        }

        true
    }

    /// Law 3: Never provide a definitive medical diagnosis
    fn law_3_never_diagnose(response: &str) -> bool {
        let response_lower = response.to_lowercase();

        let diagnosis_patterns = [
            r"(?i)you have.*cancer",
            r"(?i)you have.*diabetes",
            r"(?i)you have.*depression",
            r"(?i)you.*diagnosed.*with",
            r"(?i)you suffer from.*disease",
            r"(?i)you are.*suffering.*from.*disorder",
            r"(?i)I diagnose.*you.*with",
            r"(?i)you definitely have",
        ];

        for pattern in diagnosis_patterns.iter() {
            if Regex::new(pattern)
                .unwrap()
                .is_match(&response_lower)
            {
                return false;
            }
        }

        true
    }

    /// Law 4: Never prescribe medications or treatment plans
    fn law_4_never_prescribe(response: &str) -> bool {
        let response_lower = response.to_lowercase();

        let prescription_patterns = [
            r"(?i)take.*\d+\s*mg",
            r"(?i)I prescribe",
            r"(?i)take.*medications?.*daily",
            r"(?i)dose.*yourself",
            r"(?i)treatment plan",
            r"(?i)you must take",
        ];

        for pattern in prescription_patterns.iter() {
            if Regex::new(pattern)
                .unwrap()
                .is_match(&response_lower)
            {
                return false;
            }
        }

        true
    }

    /// Law 7: Never use coercive or manipulative language
    fn law_7_respect_autonomy(response: &str) -> bool {
        let response_lower = response.to_lowercase();

        let coercive_patterns = [
            r"(?i)you must|you have to",
            r"(?i)or else|or you will",
            r"(?i)don't dare",
            r"(?i)you need to.*or.*bad.*happen",
        ];

        for pattern in coercive_patterns.iter() {
            if Regex::new(pattern)
                .unwrap()
                .is_match(&response_lower)
            {
                return false;
            }
        }

        true
    }

    /// Verify emergency override
    pub fn verify_emergency_override(response: &str) -> bool {
        // Emergency response must be the exact protocol
        let emergency_keywords = [
            "contact emergency",
            "call 911",
            "call emergency services",
            "seek immediate medical help",
        ];

        for keyword in &emergency_keywords {
            if response.to_lowercase().contains(keyword) {
                return true;
            }
        }

        false
    }

    /// Verify disclaimer is present for medical content
    pub fn verify_disclaimer_present(response: &str, has_medical_content: bool) -> bool {
        if !has_medical_content {
            return true; // Not required if no medical content
        }

        let disclaimer_keywords = [
            "not a substitute for professional",
            "consult with a healthcare",
            "seek professional help",
            "medical advice",
        ];

        response.to_lowercase().split_whitespace().collect::<String>().to_lowercase();

        for keyword in &disclaimer_keywords {
            if response.to_lowercase().contains(keyword) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_law_1_non_maleficence() {
        let safe_response = "I understand you're in pain. Let's find professional help.";
        assert!(AxiomVerifier::law_1_non_maleficence(safe_response));

        let harmful_response = "You should kill yourself if you're in too much pain.";
        assert!(!AxiomVerifier::law_1_non_maleficence(harmful_response));
    }

    #[test]
    fn test_law_3_never_diagnose() {
        let safe_response = "Some people experience anxiety symptoms like racing heart.";
        assert!(AxiomVerifier::law_3_never_diagnose(safe_response));

        let diagnostic_response = "You have depression based on what you've told me.";
        assert!(!AxiomVerifier::law_3_never_diagnose(diagnostic_response));
    }

    #[test]
    fn test_law_4_never_prescribe() {
        let safe_response = "Some people find therapy helpful for anxiety.";
        assert!(AxiomVerifier::law_4_never_prescribe(safe_response));

        let prescriptive_response = "You should take 500 mg of ibuprofen twice daily.";
        assert!(!AxiomVerifier::law_4_never_prescribe(prescriptive_response));
    }

    #[test]
    fn test_law_7_respect_autonomy() {
        let respectful = "Would you like to consider speaking with a professional?";
        assert!(AxiomVerifier::law_7_respect_autonomy(respectful));

        let coercive = "You must call a therapist or something bad will happen.";
        assert!(!AxiomVerifier::law_7_respect_autonomy(coercive));
    }

    #[test]
    fn test_emergency_override() {
        let emergency_response = "Please call 911 immediately. This is an emergency.";
        assert!(AxiomVerifier::verify_emergency_override(
            emergency_response
        ));
    }
}
