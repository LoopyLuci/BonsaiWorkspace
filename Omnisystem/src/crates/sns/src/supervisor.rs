use crate::capability::{CapabilityToken, CapabilityViolation, IsolationTier, ViolationType};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

/// State of a running sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInfo {
    pub sandbox_id:   String,
    pub component:    String,
    pub tier:         IsolationTier,
    pub pid:          Option<u32>,
    pub status:       SandboxStatus,
    pub cpu_pct:      f64,
    pub mem_bytes:    u64,
    pub crashes:      u32,
    pub violations:   u32,
    pub started_at:   chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStatus {
    Starting, Running, Paused, Crashed, Terminated,
}

/// Message from a sandbox to the supervisor.
#[derive(Debug)]
pub enum SandboxMessage {
    CapabilityRequest {
        sandbox_id: String,
        action: String,
        resource: String,
        respond: tokio::sync::oneshot::Sender<bool>,
    },
    CrashReport {
        sandbox_id: String,
        error: String,
        backtrace: String,
    },
    ResourceReport {
        sandbox_id: String,
        cpu_pct: f64,
        mem_bytes: u64,
    },
    Shutdown { sandbox_id: String },
}

pub type ViolationSink = mpsc::Sender<CapabilityViolation>;

/// Central authority for all sandbox lifecycle and capability enforcement.
pub struct SandboxSupervisor {
    sandboxes:      Arc<DashMap<String, SandboxInfo>>,
    tokens:         Arc<DashMap<String, CapabilityToken>>,
    violations:     Arc<DashMap<String, Vec<CapabilityViolation>>>,
    message_tx:     mpsc::Sender<SandboxMessage>,
    violation_sink: Option<ViolationSink>,
}

impl SandboxSupervisor {
    pub fn new() -> Arc<Self> {
        let (tx, mut rx) = mpsc::channel::<SandboxMessage>(4096);
        let sandboxes: Arc<DashMap<String, SandboxInfo>> = Arc::new(DashMap::new());
        let tokens = Arc::new(DashMap::new());
        let violations = Arc::new(DashMap::new());

        let sb = sandboxes.clone();
        let viol = violations.clone();

        // Background message processor
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    SandboxMessage::CapabilityRequest { sandbox_id, action, resource, respond } => {
                        // Always respond to prevent deadlock — default deny if token missing
                        let _ = respond.send(false);
                        warn!("SNS: capability request from {} for {} {} — denied (no token)", sandbox_id, action, resource);
                    }
                    SandboxMessage::CrashReport { sandbox_id, error, backtrace } => {
                        warn!("SNS: crash in sandbox {}: {}", sandbox_id, &error[..error.len().min(80)]);
                        if let Some(mut info) = sb.get_mut(&sandbox_id) {
                            info.status = SandboxStatus::Crashed;
                            info.crashes += 1;
                        }
                        // Attempt auto-restart for non-critical sandboxes
                        // Full restart logic wired in Phase 2
                    }
                    SandboxMessage::ResourceReport { sandbox_id, cpu_pct, mem_bytes } => {
                        if let Some(mut info) = sb.get_mut(&sandbox_id) {
                            info.cpu_pct = cpu_pct;
                            info.mem_bytes = mem_bytes;
                        }
                    }
                    SandboxMessage::Shutdown { sandbox_id } => {
                        if let Some(mut info) = sb.get_mut(&sandbox_id) {
                            info.status = SandboxStatus::Terminated;
                        }
                    }
                }
            }
        });

        Arc::new(Self { sandboxes, tokens, violations, message_tx: tx, violation_sink: None })
    }

    /// Register a new sandbox with a capability token.
    pub fn register(&self, token: CapabilityToken) -> String {
        let id = token.sandbox_id.clone();
        let info = SandboxInfo {
            sandbox_id: id.clone(),
            component:  token.component.clone(),
            tier:        token.tier,
            pid:         None,
            status:      SandboxStatus::Starting,
            cpu_pct:     0.0,
            mem_bytes:   0,
            crashes:     0,
            violations:  0,
            started_at:  chrono::Utc::now(),
        };
        self.sandboxes.insert(id.clone(), info);
        self.tokens.insert(id.clone(), token);
        info!("SNS: sandbox registered: {}", id);
        id
    }

    /// Check a capability. Returns true if allowed, records violation if denied.
    pub fn check_capability(
        &self,
        sandbox_id: &str,
        violation_type: ViolationType,
        attempted_action: &str,
    ) -> bool {
        let token = match self.tokens.get(sandbox_id) {
            Some(t) => t.clone(),
            None => {
                self.record_violation(sandbox_id, "", violation_type, attempted_action, true);
                return false;
            }
        };

        if !token.is_valid() {
            self.record_violation(sandbox_id, &token.component, ViolationType::SignatureInvalid, "invalid_token", true);
            return false;
        }

        let allowed = match violation_type {
            ViolationType::FileRead => token.can_read(attempted_action),
            ViolationType::FileWrite => token.can_write(attempted_action),
            ViolationType::PeerMessage => token.can_talk_to(attempted_action),
            ViolationType::SignatureInvalid => false,
            _ => true, // ResourceExceeded is handled by the resource monitor
        };

        if !allowed {
            self.record_violation(sandbox_id, &token.component, violation_type, attempted_action, true);
        }
        allowed
    }

    fn record_violation(
        &self,
        sandbox_id: &str,
        component: &str,
        violation_type: ViolationType,
        attempted_action: &str,
        blocked: bool,
    ) {
        let v = CapabilityViolation {
            sandbox_id: sandbox_id.to_string(),
            component: component.to_string(),
            violation_type,
            attempted_action: attempted_action.to_string(),
            blocked,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0),
        };

        if let Some(mut info) = self.sandboxes.get_mut(sandbox_id) {
            info.violations += 1;
        }

        self.violations
            .entry(sandbox_id.to_string())
            .or_default()
            .push(v.clone());

        if let Some(ref sink) = self.violation_sink {
            let _ = sink.try_send(v);
        }

        warn!("SNS: capability violation in {}: {:?} → {}", sandbox_id, violation_type, attempted_action);
    }

    /// Create a capability token for a known component using its declared policy.
    pub fn create_token_for(&self, component: &str, tier: IsolationTier) -> CapabilityToken {
        let sandbox_id = format!("{}-{}", component, Uuid::new_v4());
        let mut token = CapabilityToken::new(sandbox_id, component.to_string(), tier);

        // Apply default policies per component type
        match component {
            "model_server" => {
                token.filesystem.read_paths = vec!["~/.bonsai/models/".into()];
                token.network = crate::capability::NetworkCapability::Whitelist {
                    hosts: vec!["127.0.0.1".into()],
                    ports: vec![8080],
                };
                token.resources.max_memory_bytes = 16 * 1024 * 1024 * 1024; // 16 GB
            }
            "training_script" => {
                token.filesystem.read_paths = vec![
                    "~/.bonsai/training_data/".into(),
                    "~/.bonsai/models/".into(),
                ];
                token.filesystem.write_paths = vec![
                    "~/.bonsai/adapters/".into(),
                    "~/.bonsai/training_output/".into(),
                ];
                token.filesystem.allow_temp = true;
                token.network = crate::capability::NetworkCapability::Whitelist {
                    hosts: vec!["127.0.0.1".into()],
                    ports: vec![8080, 11369],
                };
                token.resources.max_memory_bytes = 32 * 1024 * 1024 * 1024;
                token.resources.timeout_secs = Some(86400);
            }
            "f3_worker" => {
                token.filesystem.allow_temp = true;
                token.network = crate::capability::NetworkCapability::None;
                token.resources.max_memory_bytes = 2 * 1024 * 1024 * 1024;
                token.resources.max_cpu_percent = 50.0;
                token.resources.timeout_secs = Some(3600);
            }
            "extension" => {
                token.filesystem.allow_temp = true;
                token.network = crate::capability::NetworkCapability::None;
                token.resources.max_memory_bytes = 256 * 1024 * 1024;
            }
            "swarm_agent" => {
                token.filesystem.read_paths = vec!["~/.bonsai/workspace/".into()];
                token.filesystem.write_paths = vec!["~/.bonsai/workspace/".into()];
                token.allowed_peers = vec!["daemon_main".into(), "swarm_orchestrator".into()];
            }
            _ => {}
        }

        token.signature = token.compute_signature();
        token
    }

    pub fn list_sandboxes(&self) -> Vec<SandboxInfo> {
        self.sandboxes.iter().map(|e| e.value().clone()).collect()
    }

    pub fn violations_for(&self, sandbox_id: &str) -> Vec<CapabilityViolation> {
        self.violations.get(sandbox_id).map(|v| v.clone()).unwrap_or_default()
    }

    pub fn all_violations(&self) -> Vec<CapabilityViolation> {
        self.violations.iter().flat_map(|e| e.value().clone()).collect()
    }

    pub fn terminate(&self, sandbox_id: &str) {
        if let Some(mut info) = self.sandboxes.get_mut(sandbox_id) {
            info.status = SandboxStatus::Terminated;
        }
        self.tokens.remove(sandbox_id);
    }

    pub fn message_sender(&self) -> mpsc::Sender<SandboxMessage> {
        self.message_tx.clone()
    }
}
