// Printer type definitions and identification

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported 3D printer technologies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrinterType {
    /// Fused Deposition Modeling (FDM) - most common
    FDM,
    /// Stereolithography (SLA) - resin-based
    SLA,
    /// Selective Laser Sintering (SLS)
    SLS,
    /// Binder Jetting
    BinderJetting,
    /// PolyJet (Stratasys multi-material)
    PolyJet,
    /// Direct Metal Laser Sintering (DMLS)
    DMLS,
    /// Multi-Jet Fusion (MJF)
    MultiJetFusion,
    /// Powder Bed Fusion (PBF)
    PowderBedFusion,
    /// Digital Light Processing (DLP)
    DLP,
    /// Laminated Object Manufacturing (LOM)
    LOM,
}

impl fmt::Display for PrinterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrinterType::FDM => write!(f, "FDM"),
            PrinterType::SLA => write!(f, "SLA"),
            PrinterType::SLS => write!(f, "SLS"),
            PrinterType::BinderJetting => write!(f, "Binder Jetting"),
            PrinterType::PolyJet => write!(f, "PolyJet"),
            PrinterType::DMLS => write!(f, "DMLS"),
            PrinterType::MultiJetFusion => write!(f, "Multi-Jet Fusion"),
            PrinterType::PowderBedFusion => write!(f, "Powder Bed Fusion"),
            PrinterType::DLP => write!(f, "DLP"),
            PrinterType::LOM => write!(f, "LOM"),
        }
    }
}

/// Printer brand/manufacturer
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManufacturerBrand {
    // FDM Manufacturers
    Prusa,
    Creality,
    Anycubic,
    Elegoo,
    Tevo,
    Ultimaker,
    Stratasys,
    HP3D,
    EOS,
    Other(String),
}

impl fmt::Display for ManufacturerBrand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManufacturerBrand::Prusa => write!(f, "Prusa"),
            ManufacturerBrand::Creality => write!(f, "Creality"),
            ManufacturerBrand::Anycubic => write!(f, "Anycubic"),
            ManufacturerBrand::Elegoo => write!(f, "Elegoo"),
            ManufacturerBrand::Tevo => write!(f, "Tevo"),
            ManufacturerBrand::Ultimaker => write!(f, "Ultimaker"),
            ManufacturerBrand::Stratasys => write!(f, "Stratasys"),
            ManufacturerBrand::HP3D => write!(f, "HP 3D"),
            ManufacturerBrand::EOS => write!(f, "EOS"),
            ManufacturerBrand::Other(name) => write!(f, "{}", name),
        }
    }
}

/// Unique printer identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterIdentity {
    /// Unique printer ID (UUID or serial number)
    pub id: String,
    /// Model name (e.g., "Prusa i3 MK3S+")
    pub model: String,
    /// Brand/manufacturer
    pub manufacturer: ManufacturerBrand,
    /// Hardware version
    pub hw_version: String,
    /// Firmware version
    pub fw_version: String,
    /// Serial number
    pub serial_number: String,
}

/// Printer information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInfo {
    /// Printer identity
    pub identity: PrinterIdentity,
    /// Printer type
    pub printer_type: PrinterType,
    /// Build plate dimensions (X, Y, Z) in mm
    pub build_volume: (f32, f32, f32),
    /// Maximum hot-end temperature (°C)
    pub max_hotend_temp: u16,
    /// Maximum bed temperature (°C)
    pub max_bed_temp: u16,
    /// Number of extruders/nozzles
    pub num_extruders: u8,
    /// Supports multi-material printing
    pub multi_material: bool,
    /// Supports heated chamber
    pub heated_chamber: bool,
    /// Maximum print speed (mm/s)
    pub max_print_speed: u16,
    /// Features (e.g., "auto-leveling", "wifi")
    pub features: Vec<String>,
}

impl PrinterInfo {
    /// Create new printer info
    pub fn new(
        identity: PrinterIdentity,
        printer_type: PrinterType,
        build_volume: (f32, f32, f32),
    ) -> Self {
        Self {
            identity,
            printer_type,
            build_volume,
            max_hotend_temp: 300,
            max_bed_temp: 120,
            num_extruders: 1,
            multi_material: false,
            heated_chamber: false,
            max_print_speed: 200,
            features: Vec::new(),
        }
    }

    /// Add a feature
    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.features.push(feature.into());
        self
    }

    /// Set maximum temperatures
    pub fn with_max_temps(mut self, hotend: u16, bed: u16) -> Self {
        self.max_hotend_temp = hotend;
        self.max_bed_temp = bed;
        self
    }

    /// Set number of extruders
    pub fn with_extruders(mut self, count: u8) -> Self {
        self.num_extruders = count;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_printer_type_display() {
        assert_eq!(PrinterType::FDM.to_string(), "FDM");
        assert_eq!(PrinterType::SLA.to_string(), "SLA");
    }

    #[test]
    fn test_printer_identity() {
        let identity = PrinterIdentity {
            id: "printer-001".to_string(),
            model: "Prusa i3 MK3S+".to_string(),
            manufacturer: ManufacturerBrand::Prusa,
            hw_version: "2.0".to_string(),
            fw_version: "3.12.0".to_string(),
            serial_number: "SN12345".to_string(),
        };

        assert_eq!(identity.model, "Prusa i3 MK3S+");
        assert_eq!(identity.manufacturer, ManufacturerBrand::Prusa);
    }

    #[test]
    fn test_printer_info() {
        let identity = PrinterIdentity {
            id: "printer-001".to_string(),
            model: "Ender 3".to_string(),
            manufacturer: ManufacturerBrand::Creality,
            hw_version: "1.0".to_string(),
            fw_version: "2.0.6".to_string(),
            serial_number: "ER123456".to_string(),
        };

        let info = PrinterInfo::new(identity, PrinterType::FDM, (220.0, 220.0, 250.0))
            .with_feature("auto-leveling")
            .with_feature("wifi")
            .with_max_temps(280, 100)
            .with_extruders(1);

        assert_eq!(info.printer_type, PrinterType::FDM);
        assert_eq!(info.build_volume, (220.0, 220.0, 250.0));
        assert_eq!(info.features.len(), 2);
        assert_eq!(info.max_hotend_temp, 280);
    }
}
