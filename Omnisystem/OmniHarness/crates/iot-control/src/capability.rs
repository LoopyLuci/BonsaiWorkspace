use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
    pub readable: bool,
    pub writable: bool,
    pub min_value: Option<f32>,
    pub max_value: Option<f32>,
}

impl Capability {
    pub fn new(
        name: String,
        description: String,
        readable: bool,
        writable: bool,
    ) -> Self {
        Capability {
            name,
            description,
            readable,
            writable,
            min_value: None,
            max_value: None,
        }
    }

    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }

    pub fn is_valid(&self) -> bool {
        if self.readable || self.writable {
            true
        } else {
            false
        }
    }

    pub fn validate_value(&self, value: f32) -> bool {
        if let (Some(min), Some(max)) = (self.min_value, self.max_value) {
            value >= min && value <= max
        } else {
            true
        }
    }
}

// Common capability presets
pub mod presets {
    use super::*;

    pub fn power() -> Capability {
        Capability::new(
            "power".to_string(),
            "Turn device on/off".to_string(),
            true,
            true,
        )
    }

    pub fn brightness() -> Capability {
        Capability::new(
            "brightness".to_string(),
            "Control brightness".to_string(),
            true,
            true,
        )
        .with_range(0.0, 100.0)
    }

    pub fn color_temperature() -> Capability {
        Capability::new(
            "color_temperature".to_string(),
            "Control color temperature".to_string(),
            true,
            true,
        )
        .with_range(2000.0, 6500.0)
    }

    pub fn temperature() -> Capability {
        Capability::new(
            "temperature".to_string(),
            "Read current temperature".to_string(),
            true,
            false,
        )
        .with_range(-40.0, 125.0)
    }

    pub fn humidity() -> Capability {
        Capability::new(
            "humidity".to_string(),
            "Read current humidity".to_string(),
            true,
            false,
        )
        .with_range(0.0, 100.0)
    }

    pub fn target_temperature() -> Capability {
        Capability::new(
            "target_temperature".to_string(),
            "Set target temperature".to_string(),
            true,
            true,
        )
        .with_range(10.0, 35.0)
    }

    pub fn lock() -> Capability {
        Capability::new(
            "lock".to_string(),
            "Lock/unlock device".to_string(),
            true,
            true,
        )
    }

    pub fn position() -> Capability {
        Capability::new(
            "position".to_string(),
            "Control position".to_string(),
            true,
            true,
        )
        .with_range(0.0, 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_creation() {
        let cap = Capability::new(
            "power".to_string(),
            "Turn on/off".to_string(),
            true,
            true,
        );

        assert_eq!(cap.name, "power");
        assert!(cap.readable);
        assert!(cap.writable);
    }

    #[test]
    fn test_capability_with_range() {
        let cap = Capability::new(
            "brightness".to_string(),
            "Control brightness".to_string(),
            true,
            true,
        )
        .with_range(0.0, 100.0);

        assert_eq!(cap.min_value, Some(0.0));
        assert_eq!(cap.max_value, Some(100.0));
    }

    #[test]
    fn test_validate_value_in_range() {
        let cap = presets::brightness();

        assert!(cap.validate_value(50.0));
        assert!(cap.validate_value(0.0));
        assert!(cap.validate_value(100.0));
    }

    #[test]
    fn test_validate_value_out_of_range() {
        let cap = presets::brightness();

        assert!(!cap.validate_value(-10.0));
        assert!(!cap.validate_value(150.0));
    }

    #[test]
    fn test_preset_capabilities() {
        assert!(presets::power().is_valid());
        assert!(presets::brightness().is_valid());
        assert!(presets::temperature().is_valid());
        assert!(presets::humidity().is_valid());
        assert!(presets::lock().is_valid());
    }

    #[test]
    fn test_readonly_capability() {
        let cap = Capability::new(
            "temperature".to_string(),
            "Read temperature".to_string(),
            true,
            false,
        );

        assert!(cap.readable);
        assert!(!cap.writable);
    }
}
