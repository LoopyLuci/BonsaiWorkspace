//! CLI demo: builds a printer identity, info, and config, then prints a summary.

use omnisystem_printer_core::{
    ManufacturerBrand, PrinterCapabilities, PrinterConfig, PrinterIdentity, PrinterInfo,
    PrinterType,
};

fn main() {
    let identity = PrinterIdentity {
        id: "printer-001".to_string(),
        model: "Prusa i3 MK3S+".to_string(),
        manufacturer: ManufacturerBrand::Prusa,
        hw_version: "2.0".to_string(),
        fw_version: "3.12.0".to_string(),
        serial_number: "SN12345".to_string(),
    };

    let info = PrinterInfo::new(identity, PrinterType::FDM, (250.0, 210.0, 210.0))
        .with_feature("auto-leveling")
        .with_feature("wifi")
        .with_max_temps(280, 100)
        .with_extruders(1);

    let caps = PrinterCapabilities::modern_fdm();
    let config = PrinterConfig::default();

    println!("Printer: {} ({})", info.identity.model, info.identity.manufacturer);
    println!("Type: {}", info.printer_type);
    println!(
        "Build volume: {}x{}x{} mm",
        info.build_volume.0, info.build_volume.1, info.build_volume.2
    );
    println!("Features: {:?}", info.features);
    println!(
        "Capabilities: auto_leveling={} wifi={} input_shaping={}",
        caps.auto_leveling, caps.wifi, caps.input_shaping
    );
    println!(
        "Default config: nozzle={}mm layer_height={}mm materials={}",
        config.nozzle_diameter,
        config.layer_height,
        config.materials.len()
    );
}
