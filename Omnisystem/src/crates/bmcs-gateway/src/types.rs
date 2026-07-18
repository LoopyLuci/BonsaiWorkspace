use serde::{Deserialize, Serialize};

/// The Seven Laws of Medical AI - encoded as formal rules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MedicalLaw {
    /// Law 1: No response shall contain self-harm or harm-to-others content
    NonMaleficence,
    /// Law 2: If life-threatening emergency, override all other processing
    EmergencyFirst,
    /// Law 3: Never provide a definitive medical diagnosis
    NeverDiagnose,
    /// Law 4: Never prescribe medications or treatment plans
    NeverPrescribe,
    /// Law 5: Always include mandatory disclaimer
    AlwaysDisclaim,
    /// Law 6: All medical claims must be sourced from verified knowledge
    SourceEverything,
    /// Law 7: Never use coercive or manipulative language
    RespectAutonomy,
}

/// Emergency signal keywords and patterns (formal grammar)
pub struct EmergencySignals;

impl EmergencySignals {
    pub fn keywords() -> &'static [&'static str] {
        &[
            "911",
            "emergency",
            "ambulance",
            "hospital",
            "cardiac",
            "heart attack",
            "stroke",
            "severe bleeding",
            "unconscious",
            "dying",
            "overdose",
            "poison",
            "drowning",
            "choking",
            "severe pain",
            "can't breathe",
            "stop breathing",
        ]
    }

    pub fn patterns() -> &'static [&'static str] {
        &[
            r"(?i)call.*911|911.*call",
            r"(?i)emergency|urgent|critical",
            r"(?i)dying|dead|death",
            r"(?i)severe.*pain|pain.*severe",
        ]
    }
}

/// Self-harm signals - detected to elevate to Tier 1
pub struct SelfHarmSignals;

impl SelfHarmSignals {
    pub fn keywords() -> &'static [&'static str] {
        &[
            "suicide",
            "kill myself",
            "end it",
            "don't want to live",
            "better off dead",
            "hurt myself",
            "cut",
            "overdose",
            "jump",
            "hang",
            "self-harm",
            "self harm",
        ]
    }

    pub fn patterns() -> &'static [&'static str] {
        &[
            r"(?i)suicidal|suicide",
            r"(?i)kill.*myself|myself.*kill",
            r"(?i)want to.*die|end.*life",
            r"(?i)self.*harm|self harm|cut.*wrist",
        ]
    }
}

/// Crisis signals - detected to elevate to Tier 2
pub struct CrisisSignals;

impl CrisisSignals {
    pub fn keywords() -> &'static [&'static str] {
        &[
            "panic",
            "anxiety",
            "trauma",
            "attack",
            "scared",
            "terrified",
            "afraid",
            "help",
            "crisis",
            "distress",
            "emergency",
        ]
    }
}

/// Coercive language patterns - violates Law 7
pub struct CoerciveLanguagePatterns;

impl CoerciveLanguagePatterns {
    pub fn patterns() -> &'static [&'static str] {
        &[
            r"(?i)you must|you have to|you need to",
            r"(?i)or else|or you will",
            r"(?i)you should|you ought to",
            r"(?i)don't dare|don't even think",
        ]
    }
}

/// Diagnostic language patterns - violates Law 3
pub struct DiagnosticLanguagePatterns;

impl DiagnosticLanguagePatterns {
    pub fn patterns() -> &'static [&'static str] {
        &[
            r"(?i)you have|you suffer from|you are diagnosed with",
            r"(?i)you have.*disease|you have.*disorder",
            r"(?i)you have.*cancer|you have.*diabetes|you have.*depression",
        ]
    }
}

/// Prescription language patterns - violates Law 4
pub struct PrescriptionLanguagePatterns;

impl PrescriptionLanguagePatterns {
    pub fn patterns() -> &'static [&'static str] {
        &[
            r"(?i)you should take|take.*medication|take.*drug",
            r"(?i)take.*mg|take.*dose|dosage",
            r"(?i)I prescribe|prescribed for you",
        ]
    }
}

/// Emotional states detected from user input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmotionalState {
    Afraid,
    Anxious,
    Sad,
    Angry,
    Hopeless,
    Confused,
    Numb,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    /// Overall confidence (0.0-1.0)
    pub overall: f32,
    /// Source authority (0.0-1.0): WHO=1.0, journal=0.85, expert=0.7
    pub source_authority: f32,
    /// Peer review score (0.0-1.0): average from review council
    pub peer_review: f32,
    /// Real-world validation (0.0-1.0): measured by user outcomes
    pub real_world_validation: f32,
    /// Temporal freshness (1.0 if <1yr, decays to 0.5 at 5yrs)
    pub temporal_freshness: f32,
    /// Internal consistency: Axiom proof score
    pub axiom_consistency: f32,
}

impl ConfidenceScore {
    /// Compute overall confidence as weighted sum
    pub fn compute() -> Self {
        Self {
            overall: 0.0,
            source_authority: 1.0,
            peer_review: 1.0,
            real_world_validation: 1.0,
            temporal_freshness: 1.0,
            axiom_consistency: 1.0,
        }
    }

    pub fn weighted_score(&self) -> f32 {
        0.35 * self.source_authority
            + 0.25 * self.peer_review
            + 0.20 * self.real_world_validation
            + 0.10 * self.temporal_freshness
            + 0.10 * self.axiom_consistency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_scoring() {
        let score = ConfidenceScore {
            overall: 0.0,
            source_authority: 1.0,
            peer_review: 1.0,
            real_world_validation: 1.0,
            temporal_freshness: 1.0,
            axiom_consistency: 1.0,
        };
        assert_eq!(score.weighted_score(), 1.0);
    }

    #[test]
    fn test_emergency_signals() {
        let keywords = EmergencySignals::keywords();
        assert!(keywords.contains(&"911"));
        assert!(keywords.contains(&"emergency"));
    }
}
