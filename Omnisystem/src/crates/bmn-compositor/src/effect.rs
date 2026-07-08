// Effects — transitions, filters, color grading

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Effect type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    // Color operations
    Brightness,
    Contrast,
    Saturation,
    Hue,
    Gamma,
    ColorBalance,

    // Distortion
    Blur,
    Sharpen,
    Vignette,
    Distortion,
    Perspective,

    // Artistic
    Pixelate,
    Posterize,
    Sepia,

    // Transitions
    Fade,
    Slide,
    Wipe,
    Dissolve,
    CrossFade,

    // Custom
    Custom(String),
}

/// Effect configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub name: String,
    pub effect_type: EffectType,
    pub enabled: bool,
    pub intensity: f32, // 0.0 to 1.0
    pub duration_ms: u32,
    pub parameters: HashMap<String, f32>,
}

impl Effect {
    pub fn new(name: impl Into<String>, effect_type: EffectType) -> Self {
        Self {
            name: name.into(),
            effect_type,
            enabled: true,
            intensity: 1.0,
            duration_ms: 0,
            parameters: HashMap::new(),
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.max(0.0).min(1.0);
        self
    }

    pub fn with_duration(mut self, ms: u32) -> Self {
        self.duration_ms = ms;
        self
    }

    pub fn with_parameter(mut self, key: impl Into<String>, value: f32) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    pub fn get_parameter(&self, key: &str) -> Option<f32> {
        self.parameters.get(key).copied()
    }
}

/// Transition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from_scene: String,
    pub to_scene: String,
    pub effect: Effect,
}

impl Transition {
    pub fn new(
        from_scene: impl Into<String>,
        to_scene: impl Into<String>,
        effect: Effect,
    ) -> Self {
        Self {
            from_scene: from_scene.into(),
            to_scene: to_scene.into(),
            effect,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_creation() {
        let effect = Effect::new("Fade", EffectType::Fade);
        assert_eq!(effect.name, "Fade");
        assert_eq!(effect.effect_type, EffectType::Fade);
        assert!(effect.enabled);
        assert_eq!(effect.intensity, 1.0);
    }

    #[test]
    fn test_effect_intensity() {
        let effect = Effect::new("Blur", EffectType::Blur)
            .with_intensity(0.5);

        assert_eq!(effect.intensity, 0.5);
    }

    #[test]
    fn test_effect_parameters() {
        let effect = Effect::new("Blur", EffectType::Blur)
            .with_parameter("radius", 10.5)
            .with_parameter("quality", 3.0);

        assert_eq!(effect.get_parameter("radius"), Some(10.5));
        assert_eq!(effect.get_parameter("quality"), Some(3.0));
        assert_eq!(effect.get_parameter("nonexistent"), None);
    }

    #[test]
    fn test_effect_intensity_clamping() {
        let effect1 = Effect::new("Test", EffectType::Blur)
            .with_intensity(-0.5);
        assert_eq!(effect1.intensity, 0.0);

        let effect2 = Effect::new("Test", EffectType::Blur)
            .with_intensity(1.5);
        assert_eq!(effect2.intensity, 1.0);
    }

    #[test]
    fn test_transition() {
        let effect = Effect::new("Fade", EffectType::Fade);
        let transition = Transition::new("Scene 1", "Scene 2", effect);

        assert_eq!(transition.from_scene, "Scene 1");
        assert_eq!(transition.to_scene, "Scene 2");
    }
}
