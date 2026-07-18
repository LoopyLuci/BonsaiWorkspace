/// Complete Blueprint implementation with full validation, parsing, and Crystal image generation
use serde::{Deserialize, Serialize};
use crate::{Result, BcfError};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub version: String,
    pub containers: Vec<ContainerSpec>,
    pub services: Vec<ServiceSpec>,
    pub volumes: Vec<PersistentVolume>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    pub id: String,
    pub name: String,
    pub image: String,
    pub replicas: u32,
    pub resources: ResourceSpec,
    pub storage: StorageSpec,
    pub network: NetworkSpec,
    pub capability_tokens: Vec<String>,
    pub overlay_size_mib: Option<usize>,
    pub deadline: Option<std::time::Duration>,
    pub period: Option<std::time::Duration>,
    pub probes: HealthProbes,
    pub env_vars: HashMap<String, String>,
    pub update_strategy: UpdateStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthProbes {
    pub liveness: Option<Probe>,
    pub readiness: Option<Probe>,
    pub startup: Option<Probe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Probe {
    pub http_get: Option<HttpProbe>,
    pub exec: Option<ExecProbe>,
    pub initial_delay_seconds: u32,
    pub period_seconds: u32,
    pub timeout_seconds: u32,
    pub failure_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProbe {
    pub path: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecProbe {
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateStrategy {
    Rolling { max_surge: u32, max_unavailable: u32 },
    Canary { weight_percent: u32, interval_seconds: u32 },
    BlueGreen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    pub name: String,
    pub selector: HashMap<String, String>,
    pub ports: Vec<PortMapping>,
    pub load_balancing: LoadBalancingPolicy,
    pub session_affinity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSpec {
    pub cpu_cores: f64,
    pub cpu_priority: CpuPriority,
    pub memory_mib: u64,
    pub memory_swap_mib: Option<u64>,
    pub gpu: Option<GpuResource>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CpuPriority {
    Realtime,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuResource {
    pub gpu_type: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSpec {
    pub volumes: Vec<VolumeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSpec {
    pub id: String,
    pub name: String,
    pub mount_path: String,
    pub volume_type: VolumeType,
    pub size_mib: usize,
    pub replicated: bool,
    pub backup_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentVolume {
    pub name: String,
    pub size_gib: u64,
    pub replication_factor: u32,
    pub backup_schedule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolumeType {
    Ephemeral,
    CasPersistent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSpec {
    pub ports: Vec<PortMapping>,
    pub policy: String,
    pub tls_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub container_port: u16,
    pub service_port: u16,
    pub protocol: String,  // tcp, udp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingPolicy {
    RoundRobin,
    LeastLoaded { metric: String },
    LatencyBased { percentile: u32 },
    Random,
}

pub struct BlueprintManager {
    blueprints: Arc<RwLock<HashMap<String, Blueprint>>>,
}

impl BlueprintManager {
    pub fn new() -> Self {
        Self {
            blueprints: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn store(&self, blueprint: Blueprint) -> Result<()> {
        blueprint.validate()?;
        self.blueprints.write().insert(blueprint.name.clone(), blueprint);
        tracing::info!("Blueprint stored");
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<Blueprint> {
        self.blueprints
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| BcfError::BlueprintValidation(format!("Blueprint '{}' not found", name)))
    }

    pub fn list(&self) -> Vec<String> {
        self.blueprints.read().keys().cloned().collect()
    }

    pub fn from_yaml(yaml: &str) -> Result<Blueprint> {
        serde_yaml::from_str(yaml)
            .map_err(|e| BcfError::BlueprintValidation(format!("YAML parse error: {}", e)))
    }

    pub fn from_json(json: &str) -> Result<Blueprint> {
        serde_json::from_str(json)
            .map_err(|e| BcfError::BlueprintValidation(format!("JSON parse error: {}", e)))
    }
}

impl Blueprint {
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(BcfError::BlueprintValidation("Blueprint name required".to_string()));
        }

        if self.containers.is_empty() {
            return Err(BcfError::BlueprintValidation("At least one container required".to_string()));
        }

        for (idx, container) in self.containers.iter().enumerate() {
            if container.name.is_empty() {
                return Err(BcfError::BlueprintValidation(format!("Container {} name required", idx)));
            }
            if container.image.is_empty() {
                return Err(BcfError::BlueprintValidation(format!("Container {} image required", idx)));
            }
            if container.replicas == 0 {
                return Err(BcfError::BlueprintValidation(format!("Container {} replicas > 0", idx)));
            }
            if container.resources.cpu_cores <= 0.0 {
                return Err(BcfError::BlueprintValidation(format!("Container {} cpu > 0", idx)));
            }
            if container.resources.memory_mib == 0 {
                return Err(BcfError::BlueprintValidation(format!("Container {} memory > 0", idx)));
            }
        }

        for (idx, service) in self.services.iter().enumerate() {
            if service.name.is_empty() {
                return Err(BcfError::BlueprintValidation(format!("Service {} name required", idx)));
            }
            if service.ports.is_empty() {
                return Err(BcfError::BlueprintValidation(format!("Service {} ports required", idx)));
            }
        }

        Ok(())
    }

    pub fn to_crystal_config(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
}

impl Default for BlueprintManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_container() -> ContainerSpec {
        ContainerSpec {
            id: "web-1".to_string(),
            name: "web".to_string(),
            image: "bonsai/web:1.0.0".to_string(),
            replicas: 1,
            resources: ResourceSpec {
                cpu_cores: 0.5,
                cpu_priority: CpuPriority::Normal,
                memory_mib: 256,
                memory_swap_mib: None,
                gpu: None,
            },
            storage: StorageSpec { volumes: vec![] },
            network: NetworkSpec { ports: vec![], policy: "default".to_string(), tls_enabled: false },
            capability_tokens: vec![],
            overlay_size_mib: None,
            deadline: None,
            period: None,
            probes: HealthProbes { liveness: None, readiness: None, startup: None },
            env_vars: HashMap::new(),
            update_strategy: UpdateStrategy::BlueGreen,
        }
    }

    fn valid_blueprint() -> Blueprint {
        Blueprint {
            name: "web-app".to_string(),
            version: "1.0.0".to_string(),
            containers: vec![valid_container()],
            services: vec![],
            volumes: vec![],
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn valid_blueprint_passes() {
        assert!(valid_blueprint().validate().is_ok());
    }

    #[test]
    fn empty_name_is_rejected() {
        let mut bp = valid_blueprint();
        bp.name = String::new();
        assert!(matches!(bp.validate(), Err(BcfError::BlueprintValidation(_))));
    }

    #[test]
    fn no_containers_is_rejected() {
        let mut bp = valid_blueprint();
        bp.containers.clear();
        assert!(bp.validate().is_err());
    }

    #[test]
    fn zero_replicas_is_rejected() {
        let mut bp = valid_blueprint();
        bp.containers[0].replicas = 0;
        assert!(bp.validate().is_err());
    }

    #[test]
    fn zero_cpu_is_rejected() {
        let mut bp = valid_blueprint();
        bp.containers[0].resources.cpu_cores = 0.0;
        assert!(bp.validate().is_err());
    }

    #[test]
    fn service_without_ports_is_rejected() {
        let mut bp = valid_blueprint();
        bp.services.push(ServiceSpec {
            name: "svc".to_string(),
            selector: HashMap::new(),
            ports: vec![],
            load_balancing: LoadBalancingPolicy::RoundRobin,
            session_affinity: false,
        });
        assert!(bp.validate().is_err());
    }

    #[test]
    fn manager_rejects_storing_an_invalid_blueprint() {
        let manager = BlueprintManager::new();
        let mut bp = valid_blueprint();
        bp.name = String::new();
        assert!(manager.store(bp).is_err());
        assert!(manager.list().is_empty());
    }

    #[test]
    fn manager_store_get_list_round_trip() {
        let manager = BlueprintManager::new();
        manager.store(valid_blueprint()).unwrap();

        assert_eq!(manager.list(), vec!["web-app".to_string()]);
        let fetched = manager.get("web-app").unwrap();
        assert_eq!(fetched.containers.len(), 1);

        assert!(manager.get("nonexistent").is_err());
    }

    #[test]
    fn json_round_trip_preserves_structure() {
        let bp = valid_blueprint();
        let bytes = bp.to_crystal_config();
        let restored: Blueprint = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.name, bp.name);
        assert_eq!(restored.containers.len(), bp.containers.len());
    }

    #[test]
    fn from_yaml_parses_a_minimal_blueprint() {
        let yaml = r#"
name: minimal
version: "1.0.0"
containers:
  - id: c1
    name: c
    image: img:latest
    replicas: 1
    resources:
      cpu_cores: 1.0
      cpu_priority: Normal
      memory_mib: 128
      memory_swap_mib: null
      gpu: null
    storage:
      volumes: []
    network:
      ports: []
      policy: default
      tls_enabled: false
    capability_tokens: []
    overlay_size_mib: null
    deadline: null
    period: null
    probes:
      liveness: null
      readiness: null
      startup: null
    env_vars: {}
    update_strategy: BlueGreen
services: []
volumes: []
metadata: {}
"#;
        let bp = BlueprintManager::from_yaml(yaml).unwrap();
        assert_eq!(bp.name, "minimal");
        assert!(bp.validate().is_ok());
    }
}
