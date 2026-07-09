//! CLI

use android_bridge::AndroidBridge;
use android_bridge::telemetry::TelemetryCollector;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let telemetry = TelemetryCollector::new(tx, 100);
    let bridge = AndroidBridge::new(telemetry, Duration::from_secs(30));
    bridge.initialize().await?;

    bridge
        .register_device(
            "device-1".to_string(),
            "Example Device".to_string(),
            "Pixel".to_string(),
            33,
            "192.168.1.100".to_string(),
            5037,
            "public-key".to_string(),
        )
        .await?;

    println!("Bridge fingerprint: {}", bridge.get_fingerprint());
    println!("Discovered devices: {}", bridge.get_discovered_devices().len());
    Ok(())
}
