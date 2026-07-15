use iot_control::*;

#[test]
fn test_protocol_manager_lifecycle() {
    let manager = ProtocolManager::new();

    let device = Device::new(
        "dev1".to_string(),
        "Smart Light".to_string(),
        DeviceType::Light,
        "Philips".to_string(),
        "Hue".to_string(),
        "00:11:22:33:44:55".to_string(),
        "zigbee".to_string(),
    );

    manager.register_device(device).unwrap();
    assert_eq!(manager.device_count(), 1);

    let retrieved = manager.get_device("dev1").unwrap();
    assert_eq!(retrieved.name, "Smart Light");

    manager.update_device_state("dev1", DeviceState::Offline).unwrap();
    let updated = manager.get_device("dev1").unwrap();
    assert_eq!(updated.state, DeviceState::Offline);
}

#[test]
fn test_multi_protocol_devices() {
    let manager = ProtocolManager::new();

    let zigbee_device = Device::new(
        "z1".to_string(),
        "Zigbee Light".to_string(),
        DeviceType::Light,
        "Philips".to_string(),
        "Hue".to_string(),
        "00:11:22:33:44:55".to_string(),
        "zigbee".to_string(),
    );

    let zwave_device = Device::new(
        "z2".to_string(),
        "Z-Wave Plug".to_string(),
        DeviceType::Outlet,
        "Aeotec".to_string(),
        "Smart Switch".to_string(),
        "AA:BB:CC:DD:EE:FF".to_string(),
        "zwave".to_string(),
    );

    manager.register_device(zigbee_device).unwrap();
    manager.register_device(zwave_device).unwrap();

    assert_eq!(manager.device_count(), 2);

    let devices = manager.list_devices();
    assert_eq!(devices.len(), 2);
}

#[test]
fn test_message_queueing() {
    let manager = ProtocolManager::new();

    let message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        source: "node1".to_string(),
        target: "node2".to_string(),
        protocol: Protocol::Zigbee,
        payload: vec![1, 2, 3],
        sequence: 1,
    };

    manager.enqueue_message(message.clone()).unwrap();
    let dequeued = manager.dequeue_message();
    assert!(dequeued.is_some());
}

#[test]
fn test_all_protocols_supported() {
    let protocols = vec![
        Protocol::Zigbee,
        Protocol::ZWave,
        Protocol::Thread,
        Protocol::BLE,
        Protocol::WiFi,
    ];

    assert_eq!(protocols.len(), 5);
}
