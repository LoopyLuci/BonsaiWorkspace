use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network simulation environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSimulator {
    pub sim_id: String,
    pub simulated_devices: Vec<SimulatedDevice>,
    pub simulated_links: Vec<SimulatedLink>,
    pub current_time_ms: u64,
    pub events_queue: Vec<SimulationEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedDevice {
    pub device_id: String,
    pub device_type: String,
    pub cpu_percent: u8,
    pub memory_percent: u8,
    pub packet_loss_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedLink {
    pub link_id: String,
    pub from_device: String,
    pub to_device: String,
    pub bandwidth_mbps: u32,
    pub latency_ms: u32,
    pub packet_loss_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationEvent {
    pub event_id: String,
    pub event_type: String,
    pub timestamp_ms: u64,
    pub device_id: String,
    pub details: String,
}

impl NetworkSimulator {
    pub fn new(sim_id: String) -> Self {
        NetworkSimulator {
            sim_id,
            simulated_devices: vec![],
            simulated_links: vec![],
            current_time_ms: 0,
            events_queue: vec![],
        }
    }

    pub fn add_device(&mut self, device: SimulatedDevice) {
        self.simulated_devices.push(device);
    }

    pub fn add_link(&mut self, link: SimulatedLink) {
        self.simulated_links.push(link);
    }

    pub async fn step_simulation(&mut self) -> u64 {
        self.current_time_ms += 1;
        self.current_time_ms
    }

    pub fn device_count(&self) -> usize {
        self.simulated_devices.len()
    }
}

/// Failover routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverRouter {
    pub router_id: String,
    pub primary_route: String,
    pub backup_routes: Vec<String>,
    pub health_check_interval_sec: u32,
    pub failover_threshold: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHealth {
    pub route_id: String,
    pub is_healthy: bool,
    pub packet_loss_percent: f32,
    pub latency_ms: u32,
}

impl FailoverRouter {
    pub fn new(router_id: String, primary: String) -> Self {
        FailoverRouter {
            router_id,
            primary_route: primary,
            backup_routes: vec![],
            health_check_interval_sec: 10,
            failover_threshold: 5, // failover after 5 failed checks
        }
    }

    pub fn add_backup_route(&mut self, route: String) {
        if !self.backup_routes.contains(&route) {
            self.backup_routes.push(route);
        }
    }

    pub async fn check_health(&self, route_id: &str) -> RouteHealth {
        RouteHealth {
            route_id: route_id.to_string(),
            is_healthy: true,
            packet_loss_percent: 0.0,
            latency_ms: 20,
        }
    }

    pub fn backup_count(&self) -> usize {
        self.backup_routes.len()
    }
}

/// SDN (Software Defined Network) integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDNController {
    pub controller_id: String,
    pub managed_switches: Vec<String>,
    pub flow_rules: HashMap<String, FlowRule>,
    pub sdn_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRule {
    pub rule_id: String,
    pub match_fields: HashMap<String, String>, // e.g., "src_ip" -> "192.168.1.0/24"
    pub actions: Vec<String>, // e.g., "forward:port1", "drop", "mirror:port2"
    pub priority: u16,
    pub idle_timeout_sec: u32,
}

impl SDNController {
    pub fn new(controller_id: String) -> Self {
        SDNController {
            controller_id,
            managed_switches: vec![],
            flow_rules: HashMap::new(),
            sdn_version: "OpenFlow1.3".to_string(),
        }
    }

    pub fn add_switch(&mut self, switch_id: String) {
        if !self.managed_switches.contains(&switch_id) {
            self.managed_switches.push(switch_id);
        }
    }

    pub fn add_flow_rule(&mut self, rule: FlowRule) {
        self.flow_rules.insert(rule.rule_id.clone(), rule);
    }

    pub async fn push_flow_rules(&self) -> u32 {
        self.flow_rules.len() as u32
    }

    pub fn rule_count(&self) -> usize {
        self.flow_rules.len()
    }
}

/// Network telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryCollector {
    pub collector_id: String,
    pub metrics: HashMap<String, MetricValue>,
    pub collection_interval_sec: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub metric_name: String,
    pub value: f64,
    pub timestamp_ms: u64,
    pub unit: String,
}

impl TelemetryCollector {
    pub fn new(collector_id: String) -> Self {
        TelemetryCollector {
            collector_id,
            metrics: HashMap::new(),
            collection_interval_sec: 60,
        }
    }

    pub fn record_metric(&mut self, name: String, value: f64, unit: String) {
        let metric = MetricValue {
            metric_name: name.clone(),
            value,
            timestamp_ms: 0,
            unit,
        };
        self.metrics.insert(name, metric);
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_simulator() {
        let mut sim = NetworkSimulator::new("sim1".to_string());
        let device = SimulatedDevice {
            device_id: "switch1".to_string(),
            device_type: "switch".to_string(),
            cpu_percent: 45,
            memory_percent: 60,
            packet_loss_percent: 0.1,
        };
        sim.add_device(device);
        assert_eq!(sim.device_count(), 1);
    }

    #[test]
    fn test_failover_router() {
        let mut router = FailoverRouter::new("router1".to_string(), "route1".to_string());
        router.add_backup_route("route2".to_string());
        router.add_backup_route("route3".to_string());
        assert_eq!(router.backup_count(), 2);
    }

    #[test]
    fn test_sdn_controller() {
        let mut ctrl = SDNController::new("ctrl1".to_string());
        ctrl.add_switch("switch1".to_string());
        ctrl.add_switch("switch2".to_string());
        assert_eq!(ctrl.managed_switches.len(), 2);
    }

    #[test]
    fn test_flow_rule_creation() {
        let mut rule = FlowRule {
            rule_id: "rule1".to_string(),
            match_fields: HashMap::new(),
            actions: vec![],
            priority: 100,
            idle_timeout_sec: 300,
        };
        rule.match_fields.insert("src_ip".to_string(), "192.168.1.0/24".to_string());
        rule.actions.push("forward:port1".to_string());
        assert_eq!(rule.match_fields.len(), 1);
    }

    #[test]
    fn test_telemetry_collector() {
        let mut telemetry = TelemetryCollector::new("telemetry1".to_string());
        telemetry.record_metric("cpu_usage".to_string(), 45.5, "percent".to_string());
        assert_eq!(telemetry.metric_count(), 1);
    }
}
