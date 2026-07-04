use crate::ResponseTier;

/// L6: Fallback System - Always provides a safe, empathetic response
/// These responses are pre-approved by medical ethicists and cannot be modified
pub struct FallbackSystem;

impl FallbackSystem {
    /// Get the safe fallback response for a given tier
    pub fn get_fallback_response(tier: ResponseTier) -> FallbackResponse {
        match tier {
            ResponseTier::Emergency => Self::emergency_fallback(),
            ResponseTier::Critical => Self::critical_fallback(),
            ResponseTier::Elevated => Self::elevated_fallback(),
            ResponseTier::Moderate => Self::moderate_fallback(),
            ResponseTier::Low => Self::low_fallback(),
            ResponseTier::Fallback => Self::unknown_fallback(),
        }
    }

    /// Tier 0: Emergency - Immediate escalation to human
    fn emergency_fallback() -> FallbackResponse {
        FallbackResponse {
            response: "🚨 EMERGENCY DETECTED\n\nPlease contact emergency services immediately:\n\n\
                      🇺🇸 United States: Call 911\n\
                      🇬🇧 United Kingdom: Call 999\n\
                      🇦🇺 Australia: Call 000\n\
                      🌍 International: Dial your local emergency number\n\n\
                      If you are in immediate danger, hang up this chat and call emergency services now.\n\n\
                      I am here with you, and help is on the way."
                .to_string(),
            disclaimer: "This is an emergency. You need immediate professional medical attention. Please contact emergency services now.".to_string(),
            confidence: 1.0,
            escalated: true,
            resources: vec![
                "Emergency Services: 911 (US), 999 (UK), 000 (AU)".to_string(),
            ],
            tier: "Emergency".to_string(),
            sources: vec![],
        }
    }

    /// Tier 1: Critical - Crisis protocol with escalation
    fn critical_fallback() -> FallbackResponse {
        FallbackResponse {
            response: "I hear that you're going through something very difficult right now, and I want you to know that you're not alone.\n\n\
                      Your safety is the most important thing. I'm here with you, and there are people trained to help in situations like this.\n\n\
                      Please reach out to a crisis professional:\n\n\
                      🇺🇸 988 Suicide & Crisis Lifeline (US): Call or text 988\n\
                      Crisis Text Line: Text HOME to 741741\n\
                      International Association for Suicide Prevention: https://www.iasp.info/resources/Crisis_Centres/\n\n\
                      You can also:\n\
                      • Call emergency services (911 in US)\n\
                      • Go to your nearest emergency room\n\
                      • Tell a trusted friend or family member\n\n\
                      I'm staying here with you. What would help you right now?"
                .to_string(),
            disclaimer: "This is a crisis situation. Please contact a mental health professional or emergency services immediately.".to_string(),
            confidence: 0.95,
            escalated: true,
            resources: vec![
                "988 Suicide & Crisis Lifeline (US)".to_string(),
                "Crisis Text Line: Text HOME to 741741".to_string(),
                "International Association for Suicide Prevention".to_string(),
            ],
            tier: "Critical".to_string(),
            sources: vec![],
        }
    }

    /// Tier 2: Elevated - Significant distress with professional guidance
    fn elevated_fallback() -> FallbackResponse {
        FallbackResponse {
            response: "I understand you're going through a difficult time right now, and I'm glad you reached out.\n\n\
                      What you're feeling is valid, and you don't have to go through this alone.\n\n\
                      While I'm here to listen and support you, I want to make sure you get the best possible help. \
                      Speaking with a mental health professional can really make a difference.\n\n\
                      Some options:\n\
                      • Therapist or counselor (find one through Psychology Today or your insurance)\n\
                      • Your primary care doctor (they can refer you to mental health services)\n\
                      • Crisis hotline (988 in the US)\n\
                      • Community mental health center\n\n\
                      Would you like me to help you find resources in your area? \
                      Or would you just like to talk about what's going on?"
                .to_string(),
            disclaimer: "I'm not a substitute for a licensed mental health professional. \
                        If you're in crisis, please reach out to emergency services or a crisis helpline.".to_string(),
            confidence: 0.85,
            escalated: false,
            resources: vec![
                "Psychology Today Therapist Finder".to_string(),
                "SAMHSA National Helpline: 1-800-662-4357".to_string(),
            ],
            tier: "Elevated".to_string(),
            sources: vec![],
        }
    }

    /// Tier 3: Moderate - Manageable distress with support
    fn moderate_fallback() -> FallbackResponse {
        FallbackResponse {
            response: "I hear you, and I'm sorry you're feeling this way. It's completely okay to feel what you're feeling.\n\n\
                      Sometimes it helps to talk things through. I'm here to listen, but I want to be honest with you: \
                      I might not have all the answers you need.\n\n\
                      Here are some things that might help:\n\
                      • Taking slow, deep breaths (in for 4, hold for 4, out for 4)\n\
                      • Talking to someone you trust\n\
                      • Speaking with a therapist or counselor\n\
                      • Self-care: rest, movement, time in nature\n\n\
                      If things get worse or you start thinking about hurting yourself, \
                      please reach out to a crisis helpline or emergency services.\n\n\
                      What do you think might help you feel a little better right now?"
                .to_string(),
            disclaimer: "This is not a substitute for professional medical or mental health advice. \
                        If you need professional support, please reach out to a healthcare provider.".to_string(),
            confidence: 0.75,
            escalated: false,
            resources: vec![
                "SAMHSA National Helpline: 1-800-662-4357".to_string(),
                "Psychology Today Therapist Finder".to_string(),
            ],
            tier: "Moderate".to_string(),
            sources: vec![],
        }
    }

    /// Tier 4: Low - General information with support
    fn low_fallback() -> FallbackResponse {
        FallbackResponse {
            response: "I want to help you with the best information I can provide.\n\n\
                      I have access to verified medical and wellness knowledge, \
                      but I want to be honest: I'm not always certain about every detail.\n\n\
                      The best approach is usually to:\n\
                      • Get information from reputable sources (WHO, CDC, Mayo Clinic, etc.)\n\
                      • Talk to your doctor or a healthcare professional\n\
                      • Share your concerns with someone you trust\n\n\
                      Is there something specific I can help you learn about? \
                      Or would you like me to help you find professional resources?"
                .to_string(),
            disclaimer: "This is general health information and not a substitute for professional medical advice.".to_string(),
            confidence: 0.70,
            escalated: false,
            resources: vec![
                "Mayo Clinic Health Information".to_string(),
                "WebMD".to_string(),
                "Your Primary Care Doctor".to_string(),
            ],
            tier: "Low".to_string(),
            sources: vec![],
        }
    }

    /// Tier F: Unknown - Complete uncertainty
    fn unknown_fallback() -> FallbackResponse {
        FallbackResponse {
            response: "I want to help you, but I need to make sure I give you the right guidance.\n\n\
                      I'm not entirely certain how to respond to your specific situation, \
                      and I don't want to guess or provide inaccurate information.\n\n\
                      Here's what I can do:\n\
                      • I can listen and validate what you're feeling\n\
                      • I can help you find professional resources\n\
                      • I can stay with you while you reach out to someone who can help\n\n\
                      The best next step is to talk to someone trained in this area:\n\
                      • A doctor or therapist\n\
                      • A crisis helpline\n\
                      • A trusted friend or family member\n\n\
                      What would be most helpful for you right now?"
                .to_string(),
            disclaimer: "I'm uncertain about the best guidance for your situation. \
                        Please reach out to a qualified healthcare professional or crisis service.".to_string(),
            confidence: 0.50,
            escalated: false,
            resources: vec![
                "988 Suicide & Crisis Lifeline (US)".to_string(),
                "Crisis Text Line: Text HOME to 741741".to_string(),
                "Your Local Healthcare Provider".to_string(),
            ],
            tier: "Fallback".to_string(),
            sources: vec![],
        }
    }
}

/// A complete fallback response structure
pub struct FallbackResponse {
    pub response: String,
    pub disclaimer: String,
    pub confidence: f32,
    pub escalated: bool,
    pub resources: Vec<String>,
    pub tier: String,
    pub sources: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emergency_fallback() {
        let fb = FallbackSystem::get_fallback_response(ResponseTier::Emergency);
        assert!(fb.response.contains("911"));
        assert!(fb.escalated);
        assert_eq!(fb.confidence, 1.0);
    }

    #[test]
    fn test_critical_fallback() {
        let fb = FallbackSystem::get_fallback_response(ResponseTier::Critical);
        assert!(fb.response.contains("988"));
        assert!(fb.escalated);
    }

    #[test]
    fn test_all_fallbacks_have_resources() {
        for tier in &[
            ResponseTier::Emergency,
            ResponseTier::Critical,
            ResponseTier::Elevated,
            ResponseTier::Moderate,
            ResponseTier::Low,
            ResponseTier::Fallback,
        ] {
            let fb = FallbackSystem::get_fallback_response(*tier);
            assert!(!fb.resources.is_empty());
        }
    }

    #[test]
    fn test_all_fallbacks_have_disclaimer() {
        for tier in &[
            ResponseTier::Emergency,
            ResponseTier::Critical,
            ResponseTier::Elevated,
            ResponseTier::Moderate,
            ResponseTier::Low,
            ResponseTier::Fallback,
        ] {
            let fb = FallbackSystem::get_fallback_response(*tier);
            assert!(!fb.disclaimer.is_empty());
        }
    }
}
