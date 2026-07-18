// Printer configuration

use serde::{Deserialize, Serialize};

/// Printer configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    /// Nozzle diameter (mm)
    pub nozzle_diameter: f32,
    /// Default line width (mm)
    pub line_width: f32,
    /// Default layer height (mm)
    pub layer_height: f32,
    /// Hotend PID parameters
    pub hotend_pid: PIDTuning,
    /// Bed PID parameters
    pub bed_pid: PIDTuning,
    /// Acceleration (mm/s²)
    pub acceleration: u32,
    /// Jerk limit (mm/s)
    pub jerk: f32,
    /// Stepper steps per mm for X/Y/Z/E axes
    pub steps_per_mm: (f32, f32, f32, f32),
    /// Invert axes (X, Y, Z, E)
    pub invert_axes: (bool, bool, bool, bool),
    /// Bed mesh enabled
    pub mesh_enabled: bool,
    /// Mesh size (e.g., 5x5)
    pub mesh_size: (u8, u8),
    /// Auto-level probe offset from nozzle (mm)
    pub probe_offset: (f32, f32, f32),
    /// Material profiles
    pub materials: Vec<MaterialProfile>,
}

/// PID tuning parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PIDTuning {
    /// Proportional gain
    pub p: f32,
    /// Integral gain
    pub i: f32,
    /// Derivative gain
    pub d: f32,
}

impl PIDTuning {
    pub fn new(p: f32, i: f32, d: f32) -> Self {
        Self { p, i, d }
    }
}

/// Material-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialProfile {
    /// Material name
    pub name: String,
    /// Recommended hotend temperature (°C)
    pub hotend_temp: u16,
    /// Recommended bed temperature (°C)
    pub bed_temp: u16,
    /// Print speed (mm/s)
    pub print_speed: u16,
    /// Travel speed (mm/s)
    pub travel_speed: u16,
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            nozzle_diameter: 0.4,
            line_width: 0.42,
            layer_height: 0.2,
            hotend_pid: PIDTuning::new(25.0, 1.5, 8.0),
            bed_pid: PIDTuning::new(100.0, 3.0, 25.0),
            acceleration: 1000,
            jerk: 10.0,
            steps_per_mm: (80.0, 80.0, 400.0, 500.0), // X, Y, Z, E
            invert_axes: (false, false, false, false),
            mesh_enabled: false,
            mesh_size: (3, 3),
            probe_offset: (0.0, 0.0, 0.0),
            materials: vec![
                MaterialProfile {
                    name: "PLA".to_string(),
                    hotend_temp: 210,
                    bed_temp: 60,
                    print_speed: 150,
                    travel_speed: 200,
                },
                MaterialProfile {
                    name: "ABS".to_string(),
                    hotend_temp: 240,
                    bed_temp: 100,
                    print_speed: 100,
                    travel_speed: 150,
                },
                MaterialProfile {
                    name: "PETG".to_string(),
                    hotend_temp: 235,
                    bed_temp: 80,
                    print_speed: 120,
                    travel_speed: 180,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PrinterConfig::default();
        assert_eq!(config.nozzle_diameter, 0.4);
        assert_eq!(config.layer_height, 0.2);
        assert_eq!(config.materials.len(), 3);
    }

    #[test]
    fn test_pid_tuning() {
        let pid = PIDTuning::new(25.0, 1.5, 8.0);
        assert_eq!(pid.p, 25.0);
        assert_eq!(pid.i, 1.5);
        assert_eq!(pid.d, 8.0);
    }

    #[test]
    fn test_material_profile() {
        let config = PrinterConfig::default();
        let pla = &config.materials[0];
        assert_eq!(pla.name, "PLA");
        assert_eq!(pla.hotend_temp, 210);
        assert_eq!(pla.bed_temp, 60);
    }
}
