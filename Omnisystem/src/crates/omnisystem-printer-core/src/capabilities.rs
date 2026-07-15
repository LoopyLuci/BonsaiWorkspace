// Printer capabilities and feature detection

use serde::{Deserialize, Serialize};

/// Printer capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterCapabilities {
    /// Supports auto bed leveling
    pub auto_leveling: bool,
    /// Supports leveling mesh (e.g., bilinear)
    pub mesh_leveling: bool,
    /// Supports sensorless homing
    pub sensorless_homing: bool,
    /// Supports filament sensor
    pub filament_sensor: bool,
    /// Supports power loss recovery
    pub power_recovery: bool,
    /// Supports multiple materials simultaneously
    pub multi_material: bool,
    /// Supports heated chamber
    pub heated_chamber: bool,
    /// Supports tool changer
    pub tool_changer: bool,
    /// Supports direct drive extrusion
    pub direct_drive: bool,
    /// Supports Bowden extrusion
    pub bowden: bool,
    /// Supports linear advance (pressure compensation)
    pub linear_advance: bool,
    /// Supports input shaping (resonance cancellation)
    pub input_shaping: bool,
    /// Supports adaptive mesh leveling
    pub adaptive_mesh: bool,
    /// Maximum concurrent heaters
    pub max_heaters: u8,
    /// Maximum concurrent fans
    pub max_fans: u8,
    /// Supports SD card
    pub sd_card: bool,
    /// Supports WiFi connectivity
    pub wifi: bool,
    /// Supports Ethernet
    pub ethernet: bool,
    /// Supports Bluetooth
    pub bluetooth: bool,
    /// Supports camera module
    pub camera: bool,
    /// Supports load cell for bed leveling
    pub load_cell: bool,
}

impl PrinterCapabilities {
    /// Create default capabilities (minimal printer)
    pub fn minimal() -> Self {
        Self {
            auto_leveling: false,
            mesh_leveling: false,
            sensorless_homing: false,
            filament_sensor: false,
            power_recovery: false,
            multi_material: false,
            heated_chamber: false,
            tool_changer: false,
            direct_drive: true,
            bowden: false,
            linear_advance: false,
            input_shaping: false,
            adaptive_mesh: false,
            max_heaters: 1,
            max_fans: 1,
            sd_card: false,
            wifi: false,
            ethernet: false,
            bluetooth: false,
            camera: false,
            load_cell: false,
        }
    }

    /// Create capabilities for modern FDM printer (Prusa-like)
    pub fn modern_fdm() -> Self {
        Self {
            auto_leveling: true,
            mesh_leveling: true,
            sensorless_homing: true,
            filament_sensor: true,
            power_recovery: true,
            multi_material: false,
            heated_chamber: false,
            tool_changer: false,
            direct_drive: false,
            bowden: true,
            linear_advance: true,
            input_shaping: true,
            adaptive_mesh: true,
            max_heaters: 2,
            max_fans: 3,
            sd_card: true,
            wifi: true,
            ethernet: false,
            bluetooth: true,
            camera: true,
            load_cell: true,
        }
    }

    /// Create capabilities for high-end multi-material printer
    pub fn multi_material() -> Self {
        let mut caps = Self::modern_fdm();
        caps.multi_material = true;
        caps.tool_changer = true;
        caps.max_heaters = 4;
        caps.max_fans = 6;
        caps
    }

    /// Check if a specific capability is enabled
    pub fn has(&self, capability: &str) -> bool {
        match capability {
            "auto_leveling" => self.auto_leveling,
            "mesh_leveling" => self.mesh_leveling,
            "sensorless_homing" => self.sensorless_homing,
            "filament_sensor" => self.filament_sensor,
            "power_recovery" => self.power_recovery,
            "multi_material" => self.multi_material,
            "heated_chamber" => self.heated_chamber,
            "tool_changer" => self.tool_changer,
            "direct_drive" => self.direct_drive,
            "bowden" => self.bowden,
            "linear_advance" => self.linear_advance,
            "input_shaping" => self.input_shaping,
            "adaptive_mesh" => self.adaptive_mesh,
            "sd_card" => self.sd_card,
            "wifi" => self.wifi,
            "ethernet" => self.ethernet,
            "bluetooth" => self.bluetooth,
            "camera" => self.camera,
            "load_cell" => self.load_cell,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_capabilities() {
        let caps = PrinterCapabilities::minimal();
        assert!(!caps.auto_leveling);
        assert!(caps.direct_drive);
        assert_eq!(caps.max_heaters, 1);
    }

    #[test]
    fn test_modern_fdm_capabilities() {
        let caps = PrinterCapabilities::modern_fdm();
        assert!(caps.auto_leveling);
        assert!(caps.mesh_leveling);
        assert!(caps.sensorless_homing);
        assert!(caps.power_recovery);
        assert!(caps.wifi);
    }

    #[test]
    fn test_has_capability() {
        let caps = PrinterCapabilities::modern_fdm();
        assert!(caps.has("auto_leveling"));
        assert!(caps.has("wifi"));
        assert!(!caps.has("unknown"));
    }

    #[test]
    fn test_multi_material_capabilities() {
        let caps = PrinterCapabilities::multi_material();
        assert!(caps.multi_material);
        assert!(caps.tool_changer);
        assert_eq!(caps.max_heaters, 4);
    }
}
