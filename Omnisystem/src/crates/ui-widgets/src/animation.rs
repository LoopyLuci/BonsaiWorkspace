use serde::{Deserialize, Serialize};

/// Animation easing function
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
}

/// Animation definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Animation {
    pub name: String,
    pub duration_ms: u32,
    pub easing: Easing,
    pub delay_ms: u32,
    pub iteration_count: u32,
    pub direction: AnimationDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnimationDirection {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

impl Animation {
    pub fn new(name: String, duration_ms: u32) -> Self {
        Animation {
            name,
            duration_ms,
            easing: Easing::Linear,
            delay_ms: 0,
            iteration_count: 1,
            direction: AnimationDirection::Normal,
        }
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn with_delay(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    pub fn infinite(mut self) -> Self {
        self.iteration_count = u32::MAX;
        self
    }

    pub fn as_css(&self) -> String {
        let easing = match self.easing {
            Easing::Linear => "linear".to_string(),
            Easing::EaseIn => "ease-in".to_string(),
            Easing::EaseOut => "ease-out".to_string(),
            Easing::EaseInOut => "ease-in-out".to_string(),
            Easing::CubicBezier(x1, y1, x2, y2) => {
                format!("cubic-bezier({}, {}, {}, {})", x1, y1, x2, y2)
            }
        };

        format!(
            "animation: {} {}ms {} {}ms {} {}",
            self.name,
            self.duration_ms,
            easing,
            self.delay_ms,
            if self.iteration_count == u32::MAX {
                "infinite".to_string()
            } else {
                self.iteration_count.to_string()
            },
            match self.direction {
                AnimationDirection::Normal => "normal",
                AnimationDirection::Reverse => "reverse",
                AnimationDirection::Alternate => "alternate",
                AnimationDirection::AlternateReverse => "alternate-reverse",
            }
        )
    }
}

/// Animation engine
pub struct AnimationEngine {
    target_fps: u32,
}

impl AnimationEngine {
    pub fn new() -> Self {
        AnimationEngine { target_fps: 60 }
    }

    pub fn get_frame_duration_ms(&self) -> u32 {
        1000 / self.target_fps
    }

    pub fn calculate_progress(&self, elapsed_ms: u32, total_ms: u32) -> f32 {
        (elapsed_ms as f32 / total_ms as f32).min(1.0)
    }

    pub fn apply_easing(&self, progress: f32, easing: Easing) -> f32 {
        match easing {
            Easing::Linear => progress,
            Easing::EaseIn => progress * progress,
            Easing::EaseOut => progress * (2.0 - progress),
            Easing::EaseInOut => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    -1.0 + (4.0 - 2.0 * progress) * progress
                }
            }
            Easing::CubicBezier(_x1, _y1, _x2, _y2) => progress, // Simplified
        }
    }
}

impl Default for AnimationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_creation() {
        let anim = Animation::new("fade-in".to_string(), 300);
        assert_eq!(anim.duration_ms, 300);
    }

    #[test]
    fn test_animation_builder() {
        let anim = Animation::new("slide-in".to_string(), 500)
            .with_easing(Easing::EaseInOut)
            .with_delay(100);

        assert_eq!(anim.delay_ms, 100);
        assert_eq!(anim.easing, Easing::EaseInOut);
    }

    #[test]
    fn test_animation_css() {
        let anim = Animation::new("fade-in".to_string(), 300);
        let css = anim.as_css();
        assert!(css.contains("fade-in"));
        assert!(css.contains("300ms"));
    }

    #[test]
    fn test_animation_engine() {
        let engine = AnimationEngine::new();
        assert_eq!(engine.target_fps, 60);
        assert_eq!(engine.get_frame_duration_ms(), 16); // 1000/60 ≈ 16ms
    }

    #[test]
    fn test_progress_calculation() {
        let engine = AnimationEngine::new();
        let progress = engine.calculate_progress(500, 1000);
        assert_eq!(progress, 0.5);
    }

    #[test]
    fn test_easing_functions() {
        let engine = AnimationEngine::new();
        let linear = engine.apply_easing(0.5, Easing::Linear);
        let ease_in = engine.apply_easing(0.5, Easing::EaseIn);
        assert!(ease_in < linear);
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
