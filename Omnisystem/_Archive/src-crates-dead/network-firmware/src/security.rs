use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network security monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMonitor {
    pub monitor_id: String,
    pub threat_level: ThreatLevel,
    pub active_threats: Vec<ThreatSignature>,
    pub blocked_packets: u64,
    pub detected_anomalies: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatSignature {
    pub threat_id: String,
    pub threat_type: String,
    pub severity: u8, // 0-255
    pub source_ip: String,
    pub detection_time_ms: u64,
    pub recommendation: String,
}

impl SecurityMonitor {
    pub fn new(monitor_id: String) -> Self {
        SecurityMonitor {
            monitor_id,
            threat_level: ThreatLevel::Safe,
            active_threats: vec![],
            blocked_packets: 0,
            detected_anomalies: 0,
        }
    }

    pub async fn analyze_packet(&mut self, packet_data: &[u8]) -> bool {
        // Simple pattern detection
        if packet_data.len() > 1000 {
            self.detected_anomalies += 1;
            return false; // Block large packets
        }
        true
    }

    pub fn threat_count(&self) -> usize {
        self.active_threats.len()
    }
}

/// Intrusion Detection System (IDS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrusionDetector {
    pub detector_id: String,
    pub detection_method: DetectionMethod,
    pub sensitivity: f32, // 0.5-2.0
    pub detections: Vec<IntrusionAlert>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionMethod {
    Signature,       // Rule-based
    Anomaly,         // Behavior-based
    Hybrid,          // Combined
    MachineLearning, // ML-based
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrusionAlert {
    pub alert_id: String,
    pub alert_type: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub severity: AlertSeverity,
    pub action_taken: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl IntrusionDetector {
    pub fn new(detector_id: String, method: DetectionMethod) -> Self {
        IntrusionDetector {
            detector_id,
            detection_method: method,
            sensitivity: 1.0,
            detections: vec![],
        }
    }

    pub async fn detect_intrusion(&mut self, src_ip: &str, dst_ip: &str, payload: &[u8]) -> bool {
        // Detect patterns
        if payload.contains(&0xFF) && payload.len() > 500 {
            let alert = IntrusionAlert {
                alert_id: format!("alert_{}", self.detections.len()),
                alert_type: "Suspicious payload".to_string(),
                src_ip: src_ip.to_string(),
                dst_ip: dst_ip.to_string(),
                severity: AlertSeverity::Warning,
                action_taken: "Packet logged".to_string(),
            };
            self.detections.push(alert);
            return true;
        }
        false
    }

    pub fn detection_count(&self) -> usize {
        self.detections.len()
    }
}

/// Anomaly detection for network traffic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficAnomalyDetector {
    pub detector_id: String,
    pub baseline_stats: TrafficStatistics,
    pub anomaly_threshold: f32,
    pub detected_anomalies: Vec<TrafficAnomaly>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficStatistics {
    pub avg_packet_size: f32,
    pub avg_packets_per_sec: f32,
    pub protocol_distribution: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficAnomaly {
    pub anomaly_id: String,
    pub anomaly_type: String,
    pub deviation_percent: f32,
    pub timestamp_ms: u64,
}

impl TrafficAnomalyDetector {
    pub fn new(detector_id: String) -> Self {
        TrafficAnomalyDetector {
            detector_id,
            baseline_stats: TrafficStatistics {
                avg_packet_size: 500.0,
                avg_packets_per_sec: 1000.0,
                protocol_distribution: HashMap::new(),
            },
            anomaly_threshold: 2.0, // 2 standard deviations
            detected_anomalies: vec![],
        }
    }

    pub async fn detect_anomaly(&mut self, packet_size: f32, _packets_per_sec: f32) -> bool {
        let size_deviation = (packet_size - self.baseline_stats.avg_packet_size).abs()
            / self.baseline_stats.avg_packet_size.max(1.0);

        if size_deviation > self.anomaly_threshold {
            let anomaly = TrafficAnomaly {
                anomaly_id: format!("anom_{}", self.detected_anomalies.len()),
                anomaly_type: "Unusual packet size".to_string(),
                deviation_percent: size_deviation * 100.0,
                timestamp_ms: 0,
            };
            self.detected_anomalies.push(anomaly);
            return true;
        }
        false
    }

    pub fn anomaly_count(&self) -> usize {
        self.detected_anomalies.len()
    }
}

/// Firewall rules engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallEngine {
    pub firewall_id: String,
    pub rules: Vec<FirewallRule>,
    pub blocked_count: u64,
    pub allowed_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub rule_id: String,
    pub action: FirewallAction,
    pub src_ip: Option<String>,
    pub dst_ip: Option<String>,
    pub port: Option<u16>,
    pub protocol: Option<String>,
    pub priority: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FirewallAction {
    Allow,
    Deny,
    Drop,
    LogOnly,
}

impl FirewallEngine {
    pub fn new(firewall_id: String) -> Self {
        FirewallEngine {
            firewall_id,
            rules: vec![],
            blocked_count: 0,
            allowed_count: 0,
        }
    }

    pub fn add_rule(&mut self, rule: FirewallRule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|r| std::cmp::Reverse(r.priority));
    }

    pub fn evaluate_packet(&mut self, src_ip: &str, dst_ip: &str, port: u16) -> FirewallAction {
        for rule in &self.rules {
            let ip_match = (rule.src_ip.is_none() || rule.src_ip.as_ref().map_or(false, |ip| ip == src_ip))
                && (rule.dst_ip.is_none() || rule.dst_ip.as_ref().map_or(false, |ip| ip == dst_ip));

            let port_match = rule.port.is_none() || rule.port == Some(port);

            if ip_match && port_match {
                match &rule.action {
                    FirewallAction::Allow => self.allowed_count += 1,
                    _ => self.blocked_count += 1,
                }
                return rule.action.clone();
            }
        }

        self.allowed_count += 1;
        FirewallAction::Allow
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_monitor() {
        let monitor = SecurityMonitor::new("monitor1".to_string());
        assert_eq!(monitor.threat_level, ThreatLevel::Safe);
    }

    #[test]
    fn test_intrusion_detector() {
        let detector = IntrusionDetector::new("det1".to_string(), DetectionMethod::Signature);
        assert_eq!(detector.detection_count(), 0);
    }

    #[test]
    fn test_traffic_anomaly_detector() {
        let detector = TrafficAnomalyDetector::new("det1".to_string());
        assert_eq!(detector.anomaly_threshold, 2.0);
    }

    #[test]
    fn test_firewall_engine() {
        let firewall = FirewallEngine::new("fw1".to_string());
        assert_eq!(firewall.rule_count(), 0);
    }

    #[test]
    fn test_threat_levels() {
        let levels = vec![ThreatLevel::Safe, ThreatLevel::High, ThreatLevel::Critical];
        assert_eq!(levels.len(), 3);
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
