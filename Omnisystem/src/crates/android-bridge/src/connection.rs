use crate::capability::{CapabilityRegistry, CapabilityToken, CapabilityType};
use crate::device::{Device, DevicePool, DeviceStatus};
use crate::discovery::{DiscoveredDevice, DiscoveryService, ManualDeviceRegistry};
use crate::error::{Error, Result};
use crate::file_sync::FileSynchronizer;
use crate::input::InputInjector;
use crate::security::{DeviceIdentity, SessionKey};
use crate::streaming::ScreenStreamer;
use crate::telemetry::{TelemetryCollector, TelemetryEvent, TelemetryEventType};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main Android Bridge orchestrator
pub struct AndroidBridge {
    /// Device pool
    device_pool: Arc<DevicePool>,
    /// Discovery service
    discovery: Arc<DiscoveryService>,
    /// Manual device registry (fallback)
    registry: Arc<ManualDeviceRegistry>,
    /// Capability registry
    capability_registry: Arc<CapabilityRegistry>,
    /// Telemetry collector
    telemetry: Arc<TelemetryCollector>,
    /// Bridge identity
    identity: DeviceIdentity,
}

impl AndroidBridge {
    /// Create new Android Bridge
    pub fn new(
        telemetry: TelemetryCollector,
        discovery_interval: std::time::Duration,
    ) -> Self {
        Self {
            device_pool: Arc::new(DevicePool::new()),
            discovery: Arc::new(DiscoveryService::new(discovery_interval)),
            registry: Arc::new(ManualDeviceRegistry::new()),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            telemetry: Arc::new(telemetry),
            identity: DeviceIdentity::generate(),
        }
    }

    /// Initialize the bridge
    pub async fn initialize(&self) -> Result<()> {
        // Start discovery service
        self.discovery.start().await?;

        // Log initialization event
        self.telemetry.record(TelemetryEvent::new(
            TelemetryEventType::Metric,
            None,
            serde_json::json!({
                "metric": "bridge_initialized",
                "bridge_fingerprint": self.identity.fingerprint,
            }),
        ));

        Ok(())
    }

    /// Manually register a device
    pub async fn register_device(
        &self,
        device_id: String,
        name: String,
        model: String,
        api_level: u32,
        ip: String,
        port: u16,
        public_key: String,
    ) -> Result<()> {
        self.registry.register(
            device_id.clone(),
            name,
            model,
            api_level,
            ip,
            port,
            public_key,
        )?;

        self.telemetry.record(TelemetryEvent::new(
            TelemetryEventType::DeviceDiscovered,
            Some(device_id),
            serde_json::json!({ "registration_type": "manual" }),
        ));

        Ok(())
    }

    /// Get all discovered devices
    pub fn get_discovered_devices(&self) -> Vec<DiscoveredDevice> {
        self.registry.get_devices()
    }

    /// Connect to device
    pub async fn connect(&self, device_id: &str) -> Result<ConnectionHandle> {
        // Find device
        let discovered = self
            .registry
            .get_device(device_id)
            .ok_or_else(|| Error::DiscoveryError("Device not found".to_string()))?;

        // Create device entry in pool
        let mut device = Device::new(
            discovered.device_id.clone(),
            discovered.name.clone(),
            discovered.model.clone(),
            discovered.api_level,
            discovered.ip.clone(),
            discovered.port,
        );

        device.mark_connected();
        device.add_capability(CapabilityType::ScreenStream);
        device.add_capability(CapabilityType::InputInjection);
        device.add_capability(CapabilityType::FileRead);
        device.add_capability(CapabilityType::FileWrite);

        self.device_pool.add_device(device.clone())?;

        // Log connection event
        self.telemetry.record(TelemetryEvent::new(
            TelemetryEventType::Connected,
            Some(device_id.to_string()),
            serde_json::json!({
                "ip": discovered.ip,
                "port": discovered.port,
            }),
        ));

        Ok(ConnectionHandle {
            device_id: device_id.to_string(),
            device,
            bridge: Arc::new(self.clone_for_handle()),
        })
    }

    /// Disconnect from device
    pub async fn disconnect(&self, device_id: &str) -> Result<()> {
        let mut device = self
            .device_pool
            .get_device(device_id)
            .ok_or_else(|| Error::InvalidState("Device not connected".to_string()))?;

        device.mark_disconnected();

        self.telemetry.record(TelemetryEvent::new(
            TelemetryEventType::Disconnected,
            Some(device_id.to_string()),
            serde_json::json!({}),
        ));

        Ok(())
    }

    /// Issue capability token
    pub async fn issue_capability(
        &self,
        device_id: &str,
        subject: &str,
        capability: CapabilityType,
        duration_hours: i64,
    ) -> Result<String> {
        let mut token = CapabilityToken::new(
            capability.clone(),
            device_id.to_string(),
            subject.to_string(),
            chrono::Utc::now() + chrono::Duration::hours(duration_hours),
            None,
        );

        // In production, would sign with proper key infrastructure
        let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::thread_rng());
        token.sign(signing_key)?;

        self.capability_registry.issue_token(token.clone())?;

        self.telemetry.record(TelemetryEvent::new(
            TelemetryEventType::CapabilityGranted,
            Some(device_id.to_string()),
            serde_json::json!({
                "subject": subject,
                "capability": format!("{:?}", capability),
                "token_id": token.id,
            }),
        ));

        Ok(token.id.to_string())
    }

    /// Revoke capability token
    pub async fn revoke_capability(&self, token_id: &str) -> Result<()> {
        let token_id = uuid::Uuid::parse_str(token_id)
            .map_err(|e| Error::CapabilityError(e.to_string()))?;

        self.capability_registry.revoke_token(token_id)?;

        self.telemetry.record(TelemetryEvent::new(
            TelemetryEventType::CapabilityRevoked,
            None,
            serde_json::json!({ "token_id": token_id }),
        ));

        Ok(())
    }

    /// Check capability
    pub fn check_capability(&self, device_id: &str, subject: &str, capability: &CapabilityType) -> bool {
        self.capability_registry.has_capability(device_id, subject, capability)
    }

    /// Get device pool
    pub fn get_device_pool(&self) -> Arc<DevicePool> {
        self.device_pool.clone()
    }

    /// Get telemetry
    pub fn get_telemetry(&self) -> Arc<TelemetryCollector> {
        self.telemetry.clone()
    }

    /// Get bridge identity fingerprint
    pub fn get_fingerprint(&self) -> String {
        self.identity.fingerprint.clone()
    }

    fn clone_for_handle(&self) -> AndroidBridge {
        AndroidBridge {
            device_pool: self.device_pool.clone(),
            discovery: self.discovery.clone(),
            registry: self.registry.clone(),
            capability_registry: self.capability_registry.clone(),
            telemetry: self.telemetry.clone(),
            identity: DeviceIdentity::generate(),
        }
    }
}

impl Clone for AndroidBridge {
    fn clone(&self) -> Self {
        Self {
            device_pool: self.device_pool.clone(),
            discovery: self.discovery.clone(),
            registry: self.registry.clone(),
            capability_registry: self.capability_registry.clone(),
            telemetry: self.telemetry.clone(),
            identity: DeviceIdentity::generate(),
        }
    }
}

/// Handle to a connected Android device
pub struct ConnectionHandle {
    /// Device ID
    pub device_id: String,
    /// Device info
    device: Device,
    /// Bridge reference
    bridge: Arc<AndroidBridge>,
}

impl ConnectionHandle {
    /// Get device info
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get mutable device info
    pub fn device_mut(&mut self) -> &mut Device {
        &mut self.device
    }

    /// Create screen streamer
    pub fn create_screen_streamer(
        &self,
        config: crate::streaming::BitrateConfig,
    ) -> Result<ScreenStreamer> {
        // Check capability
        if !self.bridge.check_capability(
            &self.device_id,
            "system",
            &CapabilityType::ScreenStream,
        ) {
            return Err(Error::CapabilityError(
                "Screen streaming not authorized".to_string(),
            ));
        }

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        Ok(ScreenStreamer::new(config, tx))
    }

    /// Create input injector
    pub fn create_input_injector(&self) -> Result<InputInjector> {
        // Check capability
        if !self.bridge.check_capability(
            &self.device_id,
            "system",
            &CapabilityType::InputInjection,
        ) {
            return Err(Error::CapabilityError(
                "Input injection not authorized".to_string(),
            ));
        }

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        Ok(InputInjector::new(tx))
    }

    /// Create file synchronizer
    pub fn create_file_synchronizer(&self, sync_root: std::path::PathBuf) -> Result<FileSynchronizer> {
        // Check capability
        if !self.bridge.check_capability(
            &self.device_id,
            "system",
            &CapabilityType::FileRead,
        ) {
            return Err(Error::CapabilityError(
                "File access not authorized".to_string(),
            ));
        }

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        FileSynchronizer::new(sync_root, tx)
    }

    /// Record metric
    pub fn record_metric(&self, name: &str, value: serde_json::Value) {
        self.bridge.telemetry.record(TelemetryEvent::new(
            TelemetryEventType::Metric,
            Some(self.device_id.clone()),
            serde_json::json!({
                "metric": name,
                "value": value,
            }),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_android_bridge_creation() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let telemetry = TelemetryCollector::new(tx, 100);
        let bridge = AndroidBridge::new(telemetry, std::time::Duration::from_secs(5));

        assert!(!bridge.get_fingerprint().is_empty());
        assert!(bridge.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_device_registration() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let telemetry = TelemetryCollector::new(tx, 100);
        let bridge = AndroidBridge::new(telemetry, std::time::Duration::from_secs(5));

        assert!(bridge
            .register_device(
                "device1".to_string(),
                "Pixel 6".to_string(),
                "Pixel 6".to_string(),
                31,
                "192.168.1.100".to_string(),
                5037,
                "pk123".to_string(),
            )
            .await
            .is_ok());

        let devices = bridge.get_discovered_devices();
        assert_eq!(devices.len(), 1);
    }

    #[tokio::test]
    async fn test_capability_issuance() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let telemetry = TelemetryCollector::new(tx, 100);
        let bridge = AndroidBridge::new(telemetry, std::time::Duration::from_secs(5));

        bridge
            .register_device(
                "device1".to_string(),
                "Pixel 6".to_string(),
                "Pixel 6".to_string(),
                31,
                "192.168.1.100".to_string(),
                5037,
                "pk123".to_string(),
            )
            .await
            .ok();

        let result = bridge
            .issue_capability(
                "device1",
                "agent1",
                CapabilityType::ScreenStream,
                24,
            )
            .await;

        assert!(result.is_ok());
    }
}
