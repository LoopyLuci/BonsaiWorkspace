use serde::{Deserialize, Serialize};

/// WCAG compliance level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WCAG {
    A,
    AA,
    AAA,
}

impl WCAG {
    pub fn as_str(&self) -> &str {
        match self {
            WCAG::A => "A",
            WCAG::AA => "AA",
            WCAG::AAA => "AAA",
        }
    }
}

/// Accessibility profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityProfile {
    pub wcag_level: WCAG,
    pub keyboard_navigable: bool,
    pub screen_reader_compatible: bool,
    pub color_contrast_ratio: f32, // 4.5:1 for AA, 7:1 for AAA
    pub font_size_scalable: bool,
    pub focus_visible: bool,
    pub aria_labels: Vec<String>,
}

impl AccessibilityProfile {
    pub fn new(wcag_level: WCAG) -> Self {
        AccessibilityProfile {
            wcag_level,
            keyboard_navigable: true,
            screen_reader_compatible: true,
            color_contrast_ratio: 7.0, // AAA level
            font_size_scalable: true,
            focus_visible: true,
            aria_labels: vec![],
        }
    }

    pub fn is_wcag_compliant(&self) -> bool {
        match self.wcag_level {
            WCAG::A => self.color_contrast_ratio >= 3.0,
            WCAG::AA => self.color_contrast_ratio >= 4.5,
            WCAG::AAA => self.color_contrast_ratio >= 7.0,
        }
    }

    pub fn add_aria_label(&mut self, label: String) {
        self.aria_labels.push(label);
    }

    pub fn get_aria_attributes(&self) -> Vec<(String, String)> {
        self.aria_labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                (
                    format!("aria-label-{}", i),
                    label.clone(),
                )
            })
            .collect()
    }
}

impl Default for AccessibilityProfile {
    fn default() -> Self {
        Self::new(WCAG::AA)
    }
}

/// Keyboard navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardNavigation {
    pub tab_order: Vec<String>,
    pub escape_closes: bool,
    pub enter_activates: bool,
    pub arrow_keys_navigate: bool,
}

impl KeyboardNavigation {
    pub fn new() -> Self {
        KeyboardNavigation {
            tab_order: vec![],
            escape_closes: true,
            enter_activates: true,
            arrow_keys_navigate: true,
        }
    }
}

impl Default for KeyboardNavigation {
    fn default() -> Self {
        Self::new()
    }
}

/// Screen reader support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenReaderSupport {
    pub aria_live: Option<String>, // polite, assertive
    pub aria_atomic: bool,
    pub aria_relevant: Vec<String>,
    pub description_id: Option<String>,
}

impl ScreenReaderSupport {
    pub fn new() -> Self {
        ScreenReaderSupport {
            aria_live: Some("polite".to_string()),
            aria_atomic: false,
            aria_relevant: vec!["additions".to_string(), "removals".to_string()],
            description_id: None,
        }
    }
}

impl Default for ScreenReaderSupport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wcag_levels() {
        assert_eq!(WCAG::A.as_str(), "A");
        assert_eq!(WCAG::AA.as_str(), "AA");
        assert_eq!(WCAG::AAA.as_str(), "AAA");
    }

    #[test]
    fn test_accessibility_profile() {
        let profile = AccessibilityProfile::new(WCAG::AA);
        assert!(profile.keyboard_navigable);
        assert!(profile.is_wcag_compliant());
    }

    #[test]
    fn test_wcag_compliance() {
        let mut profile = AccessibilityProfile::new(WCAG::AAA);
        profile.color_contrast_ratio = 6.0;
        assert!(!profile.is_wcag_compliant()); // Need 7:1 for AAA
    }

    #[test]
    fn test_aria_labels() {
        let mut profile = AccessibilityProfile::new(WCAG::AA);
        profile.add_aria_label("Close button".to_string());
        assert_eq!(profile.aria_labels.len(), 1);
    }

    #[test]
    fn test_keyboard_navigation() {
        let nav = KeyboardNavigation::new();
        assert!(nav.escape_closes);
        assert!(nav.enter_activates);
    }

    #[test]
    fn test_screen_reader_support() {
        let sr = ScreenReaderSupport::new();
        assert_eq!(sr.aria_live, Some("polite".to_string()));
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
