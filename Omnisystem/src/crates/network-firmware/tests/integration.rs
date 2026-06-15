use network_firmware::*;

#[test]
fn test_full_network_stack() {
    let layer2 = layer2::Layer2Switch::new();
    let layer3 = layer3::IPStack::new();
    let routing = routing::RoutingEngine::new();
    
    let src_mac = MacAddress::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let dst_mac = MacAddress::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x02]);
    
    layer2.learn_mac(src_mac, "eth0".to_string()).unwrap();
    layer2.learn_mac(dst_mac, "eth1".to_string()).unwrap();
    
    layer3.add_arp_entry("192.168.1.1".to_string(), src_mac.to_string()).unwrap();
    layer3.add_arp_entry("192.168.1.2".to_string(), dst_mac.to_string()).unwrap();
    
    let route = Route {
        destination: "192.168.1.0/24".to_string(),
        gateway: "192.168.1.1".to_string(),
        metric: 10,
        enabled: true,
    };
    routing.add_route(route).unwrap();
    
    assert_eq!(layer2.mac_table_size(), 2);
    assert_eq!(layer3.arp_table_size(), 2);
    assert_eq!(routing.route_count(), 1);
}

#[test]
fn test_vlan_support() {
    let vlan_manager = switching::VLANManager::new();
    
    let vlan = switching::VLANInfo {
        vlan_id: 100,
        name: "Production".to_string(),
        members: vec!["eth0".to_string(), "eth1".to_string()],
    };
    
    vlan_manager.create_vlan(vlan).unwrap();
    assert_eq!(vlan_manager.vlan_count(), 1);
    
    vlan_manager.add_member(100, "eth2".to_string()).unwrap();
}

#[test]
fn test_dhcp_allocation() {
    let dhcp = dhcp::DHCPServer::new("10.0.0.100", 20);
    
    let ip1 = dhcp.request_ip("client1");
    let ip2 = dhcp.request_ip("client2");
    
    assert!(ip1.is_some());
    assert!(ip2.is_some());
    assert_ne!(ip1, ip2);
    assert_eq!(dhcp.active_leases(), 2);
}
