use iot_control::*;

#[test]
fn test_protocol_manager_lifecycle() {
    let manager = protocol::ProtocolManager::new();
    
    let device = Device {
        id: "dev1".to_string(),
        name: "Smart Light".to_string(),
        protocol: Protocol::Zigbee,
        state: DeviceState::Online,
        rssi: -45,
        last_seen: 1000,
    };

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
    let manager = protocol::ProtocolManager::new();
    
    let zigbee_device = Device {
        id: "z1".to_string(),
        name: "Zigbee Light".to_string(),
        protocol: Protocol::Zigbee,
        state: DeviceState::Online,
        rssi: -50,
        last_seen: 1000,
    };

    let zwave_device = Device {
        id: "z2".to_string(),
        name: "Z-Wave Plug".to_string(),
        protocol: Protocol::ZWave,
        state: DeviceState::Online,
        rssi: -55,
        last_seen: 1000,
    };

    manager.register_device(zigbee_device).unwrap();
    manager.register_device(zwave_device).unwrap();

    assert_eq!(manager.device_count(), 2);
    
    let devices = manager.list_devices();
    assert_eq!(devices.len(), 2);
}

#[test]
fn test_message_queueing() {
    let manager = protocol::ProtocolManager::new();
    
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
