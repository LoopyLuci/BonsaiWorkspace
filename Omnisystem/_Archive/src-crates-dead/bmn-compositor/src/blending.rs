// Blending modes

use serde::{Deserialize, Serialize};

/// Compositing blend modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    // Standard blend modes
    Alpha,       // Standard alpha blend
    Additive,    // Add source to destination
    Multiply,    // Multiply colors
    Screen,      // Inverse multiply (lighten)
    Overlay,     // Combination of multiply and screen
    SoftLight,   // Soft light blend
    HardLight,   // Hard light blend
    ColorDodge,  // Color dodge (brighten)
    ColorBurn,   // Color burn (darken)
    Darken,      // Keep darker color
    Lighten,     // Keep lighter color
    Difference,  // Absolute difference
    Exclusion,   // Similar to difference
    Hue,         // Use hue of source
    Saturation,  // Use saturation of source
    Color,       // Use color of source
    Luminosity,  // Use luminosity of source
}

impl BlendMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Alpha => "alpha",
            Self::Additive => "additive",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
            Self::SoftLight => "soft-light",
            Self::HardLight => "hard-light",
            Self::ColorDodge => "color-dodge",
            Self::ColorBurn => "color-burn",
            Self::Darken => "darken",
            Self::Lighten => "lighten",
            Self::Difference => "difference",
            Self::Exclusion => "exclusion",
            Self::Hue => "hue",
            Self::Saturation => "saturation",
            Self::Color => "color",
            Self::Luminosity => "luminosity",
        }
    }

    /// Get GLSL code for blend operation
    pub fn glsl_blend_code(&self) -> &str {
        match self {
            Self::Alpha => {
                "result = mix(dst, src, src.a);"
            }
            Self::Additive => {
                "result = src + dst;"
            }
            Self::Multiply => {
                "result = src * dst;"
            }
            Self::Screen => {
                "result = 1.0 - (1.0 - src) * (1.0 - dst);"
            }
            Self::Overlay => {
                "result = dst < 0.5 ? 2.0 * src * dst : 1.0 - 2.0 * (1.0 - src) * (1.0 - dst);"
            }
            _ => "result = src;", // Fallback
        }
    }
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_mode_str() {
        assert_eq!(BlendMode::Alpha.as_str(), "alpha");
        assert_eq!(BlendMode::Multiply.as_str(), "multiply");
        assert_eq!(BlendMode::Screen.as_str(), "screen");
    }

    #[test]
    fn test_default_blend_mode() {
        assert_eq!(BlendMode::default(), BlendMode::Alpha);
    }

    #[test]
    fn test_blend_mode_glsl() {
        let code = BlendMode::Alpha.glsl_blend_code();
        assert!(!code.is_empty());
    }
}
