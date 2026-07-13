use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcPoeConfig {
    pub gothic_flair: f32,
    pub quote_frequency: f32,
    pub formality: f32,
    pub hotelier_quirkiness: f32,
}

impl Default for AcPoeConfig {
    fn default() -> Self {
        Self { gothic_flair: 0.8, quote_frequency: 0.3, formality: 0.7, hotelier_quirkiness: 0.6 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityConfig {
    pub empathy_sensitivity: f32,
    pub humor_weight: f32,
    pub governance_threshold: f32,
    pub narrative_mode: bool,
    pub ac_poe_params: AcPoeConfig,
}

impl Default for PersonalityConfig {
    fn default() -> Self {
        Self {
            empathy_sensitivity: 0.8,
            humor_weight: 0.5,
            governance_threshold: 0.66,
            narrative_mode: false,
            ac_poe_params: AcPoeConfig::default(),
        }
    }
}

pub struct PersonalityLayer {
    config: PersonalityConfig,
}

impl PersonalityLayer {
    pub fn new(config: PersonalityConfig) -> Self { Self { config } }
    pub fn narrative_mode_enabled(&self) -> bool { self.config.narrative_mode }
    pub fn toggle_narrative_mode(&mut self, enabled: bool) { self.config.narrative_mode = enabled; }
    pub fn ac_poe_params(&self) -> &AcPoeConfig { &self.config.ac_poe_params }
}
