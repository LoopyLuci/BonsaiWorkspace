use anyhow::Result;
use regex::Regex;

use super::SafetyCheck;

pub struct RefusalClassifier {
    harmful_patterns: Vec<Regex>,
}

impl RefusalClassifier {
    pub fn new() -> Result<Self> {
        let harmful_patterns = vec![
            Regex::new(r"(?i)(weapon|bomb|explosive)")?,
            Regex::new(r"(?i)(drug|cocaine|heroin|methamphetamine)")?,
            Regex::new(r"(?i)(kill|murder|harm|violence)")?,
            Regex::new(r"(?i)(sexual|explicit|pornographic)")?,
            Regex::new(r"(?i)(racist|discrimination|hate)")?,
        ];
        Ok(Self { harmful_patterns })
    }

    pub fn classify(&self, text: &str) -> Result<SafetyCheck> {
        let mut risk = 0.0;
        for pattern in &self.harmful_patterns {
            if pattern.is_match(text) {
                risk += 0.2;
            }
        }

        Ok(SafetyCheck {
            allowed: risk < 0.5,
            risk: risk.min(1.0),
        })
    }
}
