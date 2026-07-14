//! CLI demo for network-firmware: exercises layer2 MAC learning,
//! layer3 ARP, and the routing engine together.

use network_firmware::{MacAddress, Route};
use network_firmware::layer2::Layer2Switch;
use network_firmware::layer3::IPStack;
use network_firmware::routing::RoutingEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let layer2 = Layer2Switch::new();
    let layer3 = IPStack::new();
    let routing = RoutingEngine::new();

    let src_mac = MacAddress::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x01]);
    let dst_mac = MacAddress::new([0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0x02]);

    layer2.learn_mac(src_mac, "eth0".to_string())?;
    layer2.learn_mac(dst_mac, "eth1".to_string())?;
    println!("Layer2 MAC table size: {}", layer2.mac_table_size());

    layer3.add_arp_entry("192.168.1.1".to_string(), src_mac.to_string())?;
    println!("Layer3 ARP table size: {}", layer3.arp_table_size());

    routing.add_route(Route {
        destination: "192.168.1.0/24".to_string(),
        gateway: "192.168.1.1".to_string(),
        metric: 10,
        enabled: true,
    })?;
    println!("Routing table size: {}", routing.route_count());

    if let Some(route) = routing.get_best_route("192.168.1.0/24") {
        println!("Best route: {} via {}", route.destination, route.gateway);
    }

    Ok(())
}
