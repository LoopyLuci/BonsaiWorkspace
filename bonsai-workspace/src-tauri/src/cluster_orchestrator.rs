use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterDeviceType {
    Desktop,
    Laptop,
    Mobile,
    Tablet,
    Server,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResourceShare {
    pub cpu_share_pct: f32,
    pub ram_share_mb: u64,
    pub gpu_share_pct: f32,
    pub max_concurrency: u32,
    pub min_battery_pct: u8,
    pub allow_background_heavy_jobs: bool,
}

impl Default for NodeResourceShare {
    fn default() -> Self {
        Self {
            cpu_share_pct: 50.0,
            ram_share_mb: 4096,
            gpu_share_pct: 0.0,
            max_concurrency: 2,
            min_battery_pct: 20,
            allow_background_heavy_jobs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRuntimeMetrics {
    pub cpu_utilization_pct: f32,
    pub free_ram_mb: u64,
    pub available_gpu_pct: f32,
    pub battery_pct: Option<u8>,
    pub latency_ms: u32,
}

impl Default for NodeRuntimeMetrics {
    fn default() -> Self {
        Self {
            cpu_utilization_pct: 0.0,
            free_ram_mb: 0,
            available_gpu_pct: 0.0,
            battery_pct: None,
            latency_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    pub node_id: String,
    pub display_name: String,
    pub device_type: ClusterDeviceType,
    pub labels: Vec<String>,
    pub share: NodeResourceShare,
    pub metrics: NodeRuntimeMetrics,
    pub is_online: bool,
    pub active_workloads: u32,
    pub last_seen_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulingStrategy {
    Balanced,
    Throughput,
    LowestLatency,
    EnergySaver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterPolicy {
    pub strategy: SchedulingStrategy,
    pub max_nodes_per_workload: u8,
    pub overcommit_ratio: f32,
    pub require_label_affinity: bool,
}

impl Default for ClusterPolicy {
    fn default() -> Self {
        Self {
            strategy: SchedulingStrategy::Balanced,
            max_nodes_per_workload: 3,
            overcommit_ratio: 1.0,
            require_label_affinity: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterWorkload {
    pub workload_id: String,
    pub cpu_cost_pct: f32,
    pub ram_required_mb: u64,
    pub gpu_cost_pct: f32,
    pub latency_sensitive: bool,
    pub required_labels: Vec<String>,
    pub allow_mobile: bool,
    pub allow_desktop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadCandidate {
    pub node_id: String,
    pub score: f32,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterDispatchPlan {
    pub workload_id: String,
    pub selected: Vec<WorkloadCandidate>,
    pub rejected: Vec<WorkloadCandidate>,
    pub strategy: SchedulingStrategy,
}

#[derive(Default)]
pub struct ClusterOrchestrator {
    nodes: HashMap<String, ClusterNode>,
    policy: ClusterPolicy,
}

impl ClusterOrchestrator {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            policy: ClusterPolicy::default(),
        }
    }

    pub fn set_policy(&mut self, policy: ClusterPolicy) {
        self.policy = policy;
    }

    pub fn policy(&self) -> &ClusterPolicy {
        &self.policy
    }

    pub fn upsert_node(&mut self, node: ClusterNode) {
        self.nodes.insert(node.node_id.clone(), node);
    }

    pub fn remove_node(&mut self, node_id: &str) -> bool {
        self.nodes.remove(node_id).is_some()
    }

    pub fn update_node_metrics(&mut self, node_id: &str, metrics: NodeRuntimeMetrics, last_seen_ms: u64) -> bool {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.metrics = metrics;
            node.last_seen_ms = last_seen_ms;
            node.is_online = true;
            return true;
        }
        false
    }

    pub fn list_nodes(&self) -> Vec<ClusterNode> {
        let mut nodes: Vec<ClusterNode> = self.nodes.values().cloned().collect();
        nodes.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        nodes
    }

    pub fn plan_workload(&self, workload: &ClusterWorkload) -> ClusterDispatchPlan {
        let mut accepted: Vec<WorkloadCandidate> = Vec::new();
        let mut rejected: Vec<WorkloadCandidate> = Vec::new();

        for node in self.nodes.values() {
            let (score, rationale) = self.score_node(node, workload);
            let candidate = WorkloadCandidate {
                node_id: node.node_id.clone(),
                score,
                rationale,
            };
            if score > 0.0 {
                accepted.push(candidate);
            } else {
                rejected.push(candidate);
            }
        }

        accepted.sort_by(|a, b| b.score.total_cmp(&a.score));
        rejected.sort_by(|a, b| b.score.total_cmp(&a.score));

        let max_nodes = self.policy.max_nodes_per_workload.max(1) as usize;
        let selected = accepted.into_iter().take(max_nodes).collect();

        ClusterDispatchPlan {
            workload_id: workload.workload_id.clone(),
            selected,
            rejected,
            strategy: self.policy.strategy.clone(),
        }
    }

    fn score_node(&self, node: &ClusterNode, workload: &ClusterWorkload) -> (f32, Vec<String>) {
        let mut reasons = Vec::<String>::new();

        if !node.is_online {
            reasons.push("node offline".to_string());
            return (0.0, reasons);
        }

        if node.active_workloads >= node.share.max_concurrency.max(1) {
            reasons.push("node at max concurrency".to_string());
            return (0.0, reasons);
        }

        match node.device_type {
            ClusterDeviceType::Mobile | ClusterDeviceType::Tablet if !workload.allow_mobile => {
                reasons.push("workload disallows mobile/tablet nodes".to_string());
                return (0.0, reasons);
            }
            ClusterDeviceType::Desktop | ClusterDeviceType::Laptop | ClusterDeviceType::Server if !workload.allow_desktop => {
                reasons.push("workload disallows desktop/server nodes".to_string());
                return (0.0, reasons);
            }
            _ => {}
        }

        if !workload.required_labels.is_empty() {
            let missing: Vec<String> = workload
                .required_labels
                .iter()
                .filter(|l| !node.labels.iter().any(|n| n.eq_ignore_ascii_case(l)))
                .cloned()
                .collect();
            if !missing.is_empty() && self.policy.require_label_affinity {
                reasons.push(format!("missing required labels: {}", missing.join(", ")));
                return (0.0, reasons);
            }
            if !missing.is_empty() {
                reasons.push(format!("label mismatch penalty: {}", missing.join(", ")));
            }
        }

        if let Some(battery) = node.metrics.battery_pct {
            if battery < node.share.min_battery_pct {
                reasons.push(format!(
                    "battery {}% below allowed minimum {}%",
                    battery,
                    node.share.min_battery_pct
                ));
                return (0.0, reasons);
            }
        }

        let available_cpu = (node.share.cpu_share_pct * self.policy.overcommit_ratio) - node.metrics.cpu_utilization_pct;
        let available_ram = node.metrics.free_ram_mb.min(node.share.ram_share_mb);
        let available_gpu = (node.share.gpu_share_pct * self.policy.overcommit_ratio) - (100.0 - node.metrics.available_gpu_pct);

        if available_cpu < workload.cpu_cost_pct {
            reasons.push("insufficient CPU budget".to_string());
            return (0.0, reasons);
        }

        if available_ram < workload.ram_required_mb {
            reasons.push("insufficient RAM budget".to_string());
            return (0.0, reasons);
        }

        if workload.gpu_cost_pct > 0.0 && available_gpu < workload.gpu_cost_pct {
            reasons.push("insufficient GPU budget".to_string());
            return (0.0, reasons);
        }

        let cpu_headroom_score = ((available_cpu - workload.cpu_cost_pct) / 100.0).max(0.0);
        let ram_headroom_score = ((available_ram.saturating_sub(workload.ram_required_mb)) as f32 / 32768.0).max(0.0);
        let gpu_headroom_score = if workload.gpu_cost_pct > 0.0 {
            ((available_gpu - workload.gpu_cost_pct) / 100.0).max(0.0)
        } else {
            0.2
        };
        let latency_score = (1.0 - (node.metrics.latency_ms.min(1000) as f32 / 1000.0)).max(0.0);

        let mut score = match self.policy.strategy {
            SchedulingStrategy::Balanced => {
                cpu_headroom_score * 0.35 + ram_headroom_score * 0.25 + gpu_headroom_score * 0.15 + latency_score * 0.25
            }
            SchedulingStrategy::Throughput => {
                cpu_headroom_score * 0.45 + ram_headroom_score * 0.35 + gpu_headroom_score * 0.15 + latency_score * 0.05
            }
            SchedulingStrategy::LowestLatency => {
                cpu_headroom_score * 0.20 + ram_headroom_score * 0.10 + gpu_headroom_score * 0.10 + latency_score * 0.60
            }
            SchedulingStrategy::EnergySaver => {
                let battery_bonus = node.metrics.battery_pct.unwrap_or(100) as f32 / 100.0;
                cpu_headroom_score * 0.20 + ram_headroom_score * 0.20 + gpu_headroom_score * 0.10 + latency_score * 0.20 + battery_bonus * 0.30
            }
        };

        if workload.latency_sensitive {
            score += latency_score * 0.25;
            reasons.push(format!("latency-sensitive boost ({:.0}ms)", node.metrics.latency_ms));
        }

        reasons.push(format!(
            "cpu {:.1}% free, ram {}MB free, gpu {:.1}% free-equivalent",
            available_cpu.max(0.0),
            available_ram,
            available_gpu.max(0.0)
        ));

        if !workload.required_labels.is_empty() {
            reasons.push(format!("label match requirement: {}", workload.required_labels.join(", ")));
        }

        (score.max(0.0), reasons)
    }
}
