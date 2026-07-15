//! CLI — register a few devices and list them by type/protocol.

use iot_control::{capability::presets, Device, DeviceRegistry, DeviceType};

fn main() {
    let registry = DeviceRegistry::new();

    let mut light = Device::new(
        "device-1".to_string(),
        "Living Room Light".to_string(),
        DeviceType::Light,
        "Philips".to_string(),
        "Hue".to_string(),
        "00:11:22:33:44:55".to_string(),
        "zigbee".to_string(),
    );
    light.add_capability(presets::power()).unwrap();
    light.add_capability(presets::brightness()).unwrap();
    registry.register(light).unwrap();

    let thermostat = Device::new(
        "device-2".to_string(),
        "Hallway Thermostat".to_string(),
        DeviceType::Thermostat,
        "Honeywell".to_string(),
        "T9".to_string(),
        "AA:BB:CC:DD:EE:FF".to_string(),
        "zwave".to_string(),
    );
    registry.register(thermostat).unwrap();

    println!("Registered {} device(s):", registry.count());
    for device in registry.list_all() {
        println!(
            "  {} [{:?}] via {} — {} capabilities",
            device.name,
            device.device_type,
            device.protocol,
            device.capabilities.len()
        );
    }

    let lights = registry.list_by_type(DeviceType::Light);
    println!("Lights: {}", lights.len());
}
