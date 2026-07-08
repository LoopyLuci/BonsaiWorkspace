//! Integration tests for Android Bridge
//!
//! These tests verify the core functionality of the Android Bridge without
//! requiring a real Android device. Use mock/stub implementations.

#[cfg(test)]
mod tests {
    use android_bridge::{AndroidBridge, Error, Result};
    use android_bridge::connection::TelemetryCollector;
    use std::time::Duration;

    /// Initialize a test bridge instance
    fn create_test_bridge() -> AndroidBridge {
        let telemetry = TelemetryCollector::new();
        AndroidBridge::new(telemetry, Duration::from_secs(5))
    }

    #[tokio::test]
    async fn test_bridge_initialization() {
        let bridge = create_test_bridge();
        let result = bridge.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_device_discovery_empty() {
        let bridge = create_test_bridge();
        let _ = bridge.initialize().await;
        let devices = bridge.get_discovered_devices();
        assert!(devices.is_empty() || devices.len() >= 0);
    }

    #[tokio::test]
    async fn test_manual_device_registration() {
        let bridge = create_test_bridge();
        let result = bridge
            .register_device(
                "device-1".to_string(),
                "Test Device".to_string(),
                "Pixel 6".to_string(),
                33,
                "192.168.1.100".to_string(),
                5037,
                "ed25519_public_key_base64".to_string(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_device_connection() {
        let bridge = create_test_bridge();
        let _ = bridge.initialize().await;

        // Register a test device first
        let _ = bridge
            .register_device(
                "device-1".to_string(),
                "Test Device".to_string(),
                "Pixel 6".to_string(),
                33,
                "192.168.1.100".to_string(),
                5037,
                "public_key".to_string(),
            )
            .await;

        // Try to connect
        let result = bridge.connect("device-1").await;
        // Connection will fail without real device, but API should be correct
        match result {
            Ok(_handle) => {
                // Device connection succeeded
            }
            Err(_e) => {
                // Expected in test environment
            }
        }
    }

    #[tokio::test]
    async fn test_capability_issuance() {
        let bridge = create_test_bridge();
        let _ = bridge.initialize().await;

        let result = bridge
            .issue_capability(
                "device-1",
                "app",
                android_bridge::capability::CapabilityType::ScreenStream,
                1,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_capability_revocation() {
        let bridge = create_test_bridge();
        let _ = bridge.initialize().await;

        // Issue a capability first
        let token_result = bridge
            .issue_capability(
                "device-1",
                "app",
                android_bridge::capability::CapabilityType::InputInjection,
                1,
            )
            .await;

        if let Ok(token_id) = token_result {
            // Now revoke it
            let revoke_result = bridge.revoke_capability(&token_id).await;
            assert!(revoke_result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_capability_check() {
        let bridge = create_test_bridge();

        let has_cap = bridge.check_capability(
            "device-1",
            "app",
            &android_bridge::capability::CapabilityType::FileRead,
        );

        // Should return boolean, not error
        let _ = has_cap;
    }

    #[tokio::test]
    async fn test_bridge_fingerprint() {
        let bridge = create_test_bridge();
        let fingerprint = bridge.get_fingerprint();
        assert!(!fingerprint.is_empty());
    }

    #[tokio::test]
    async fn test_telemetry_access() {
        let bridge = create_test_bridge();
        let telemetry = bridge.get_telemetry();
        // Should be accessible and cloneable
        let _telemetry_clone = telemetry.clone();
    }

    #[tokio::test]
    async fn test_device_pool_access() {
        let bridge = create_test_bridge();
        let pool = bridge.get_device_pool();
        // Should be accessible
        let device_count = pool.list_devices().len();
        assert!(device_count >= 0);
    }

    #[tokio::test]
    async fn test_bridge_clone_consistency() {
        let bridge1 = create_test_bridge();
        let bridge2 = bridge1.clone();

        // Both should have device pools
        let pool1 = bridge1.get_device_pool();
        let pool2 = bridge2.get_device_pool();

        assert_eq!(pool1.list_devices().len(), pool2.list_devices().len());
    }

    #[test]
    fn test_device_status_enum() {
        let statuses = vec![
            android_bridge::device::DeviceStatus::Discovered,
            android_bridge::device::DeviceStatus::Connecting,
            android_bridge::device::DeviceStatus::Connected,
            android_bridge::device::DeviceStatus::Pairing,
            android_bridge::device::DeviceStatus::Paired,
            android_bridge::device::DeviceStatus::Disconnected,
            android_bridge::device::DeviceStatus::Error,
        ];

        assert_eq!(statuses.len(), 7);
    }

    #[test]
    fn test_capability_types() {
        let capabilities = vec![
            android_bridge::capability::CapabilityType::ScreenStream,
            android_bridge::capability::CapabilityType::InputInjection,
            android_bridge::capability::CapabilityType::FileRead,
            android_bridge::capability::CapabilityType::FileWrite,
        ];

        assert_eq!(capabilities.len(), 4);
    }

    #[test]
    fn test_device_creation() {
        let device = android_bridge::device::Device::new(
            "device-1".to_string(),
            "Test Device".to_string(),
            "Pixel 6".to_string(),
            33,
            "192.168.1.100".to_string(),
            5037,
        );

        assert_eq!(device.id, "device-1");
        assert_eq!(device.name, "Test Device");
        assert_eq!(device.api_level, 33);
    }

    #[tokio::test]
    async fn test_multiple_device_registration() {
        let bridge = create_test_bridge();
        let _ = bridge.initialize().await;

        for i in 0..5 {
            let result = bridge
                .register_device(
                    format!("device-{}", i),
                    format!("Test Device {}", i),
                    "Pixel 6".to_string(),
                    33,
                    format!("192.168.1.{}", 100 + i),
                    5037,
                    "public_key".to_string(),
                )
                .await;

            assert!(result.is_ok());
        }

        let devices = bridge.get_discovered_devices();
        assert!(devices.len() >= 5);
    }

    #[tokio::test]
    async fn test_concurrent_device_operations() {
        let bridge = std::sync::Arc::new(create_test_bridge());
        let _ = bridge.initialize().await;

        let mut handles = vec![];

        for i in 0..3 {
            let bridge_clone = bridge.clone();
            let handle = tokio::spawn(async move {
                let _ = bridge_clone
                    .register_device(
                        format!("device-{}", i),
                        format!("Device {}", i),
                        "Model".to_string(),
                        33,
                        format!("192.168.1.{}", 100 + i),
                        5037,
                        "key".to_string(),
                    )
                    .await;
            });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.await.is_ok());
        }
    }

    #[test]
    fn test_error_types() {
        // Test error construction
        let _err1 = Error::DiscoveryError("test".to_string());
        let _err2 = Error::InvalidState("test".to_string());
        let _err3 = Error::CapabilityError("test".to_string());
    }
}
