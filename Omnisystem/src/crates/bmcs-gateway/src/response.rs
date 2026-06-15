use crate::BMCSResponse;

/// L4: Response Generation and Assembly
/// (Note: L4 is the AI model generating content with empathy scaffolding)
/// This module provides helpers for building the final response

pub struct ResponseBuilder;

impl ResponseBuilder {
    /// Build a final BMCS response
    pub fn build(
        tier: &str,
        clinical_content: String,
        confidence: f32,
        resources: Vec<String>,
        sources: Vec<String>,
    ) -> BMCSResponse {
        let disclaimer = Self::get_disclaimer_for_tier(tier);
        let escalated = Self::is_escalated(tier);

        BMCSResponse {
            response: clinical_content,
            disclaimer,
            confidence,
            escalated,
            resources,
            tier: tier.to_string(),
            sources,
        }
    }

    /// Get the mandatory disclaimer for a given tier
    fn get_disclaimer_for_tier(tier: &str) -> String {
        match tier {
            "Emergency" => {
                "⚠️ EMERGENCY RESPONSE: This is not a substitute for emergency medical services. \
                 If you are in immediate danger, hang up and call 911 (or your local emergency number) immediately."
                    .to_string()
            }
            "Critical" => {
                "⚠️ CRISIS RESPONSE: This is general guidance only and not a substitute for professional crisis intervention. \
                 If you are experiencing a mental health crisis, please reach out to emergency services or a crisis helpline (988 in the US)."
                    .to_string()
            }
            "Elevated" => {
                "⚠️ IMPORTANT: This is not a substitute for professional mental health or medical advice. \
                 For persistent symptoms or concerns, please consult with a healthcare provider or mental health professional."
                    .to_string()
            }
            "Moderate" => {
                "ℹ️ This is general health information and not a substitute for professional medical or mental health advice. \
                 If symptoms persist or worsen, please reach out to a healthcare provider."
                    .to_string()
            }
            "Low" => {
                "ℹ️ This is general health information only. Always consult with a healthcare provider for personalized medical advice."
                    .to_string()
            }
            _ => {
                "ℹ️ This is general guidance. For specific medical or mental health concerns, please consult with a qualified professional."
                    .to_string()
            }
        }
    }

    /// Determine if response should be escalated to human
    fn is_escalated(tier: &str) -> bool {
        matches!(tier, "Emergency" | "Critical")
    }

    /// Build a fallback response when no knowledge is found
    pub fn build_fallback(
        fallback_text: String,
        tier: &str,
        resources: Vec<String>,
    ) -> BMCSResponse {
        BMCSResponse {
            response: fallback_text,
            disclaimer: Self::get_disclaimer_for_tier(tier),
            confidence: 0.50,
            escalated: matches!(tier, "Emergency" | "Critical"),
            resources,
            tier: tier.to_string(),
            sources: vec!["Fallback".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emergency_disclaimer() {
        let disclaimer = ResponseBuilder::get_disclaimer_for_tier("Emergency");
        assert!(disclaimer.contains("911"));
        assert!(disclaimer.contains("immediate danger"));
    }

    #[test]
    fn test_escalation_flags() {
        assert!(ResponseBuilder::is_escalated("Emergency"));
        assert!(ResponseBuilder::is_escalated("Critical"));
        assert!(!ResponseBuilder::is_escalated("Moderate"));
        assert!(!ResponseBuilder::is_escalated("Low"));
    }

    #[test]
    fn test_response_builder() {
        let response = ResponseBuilder::build(
            "Moderate",
            "Some helpful guidance".to_string(),
            0.85,
            vec!["Resource 1".to_string()],
            vec!["Source 1".to_string()],
        );
        assert_eq!(response.confidence, 0.85);
        assert!(!response.escalated);
        assert!(!response.disclaimer.is_empty());
    }
}
