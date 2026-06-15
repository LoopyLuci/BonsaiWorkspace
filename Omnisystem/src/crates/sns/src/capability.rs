use serde::{Deserialize, Serialize};

/// Isolation tier — determines the sandbox technology used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationTier {
    /// Tier 0: WebAssembly via wasmtime — near-native, strong isolation, no system calls.
    /// Used for: UI panels, tools, extensions, agent code, plugins.
    Wasm = 0,
    /// Tier 1: OS process with namespace isolation + seccomp BPF + dropped capabilities.
    /// Used for: trusted Bonsai daemons, watchdog, internal services.
    Process = 1,
    /// Tier 2: Container with gVisor (runsc) syscall interception.
    /// Used for: training scripts, build processes, model servers, F³ workers.
    Container = 2,
    /// Tier 3: Hardware-backed microVM (Firecracker/KVM or Hyper-V).
    /// Used for: untrusted extensions, user-submitted code, compute fabric guests.
    MicroVm = 3,
}

/// File system access granted to a sandbox.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemCapability {
    /// Absolute paths the sandbox may read.
    pub read_paths:  Vec<String>,
    /// Absolute paths the sandbox may write.
    pub write_paths: Vec<String>,
    /// May use an ephemeral temp directory.
    pub allow_temp:  bool,
    /// May access /dev/null (or Windows equivalent).
    pub allow_devnull: bool,
}

/// Network access granted to a sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkCapability {
    /// No network access.
    None,
    /// Only localhost connections.
    LocalOnly,
    /// Only the specified host:port combinations.
    Whitelist { hosts: Vec<String>, ports: Vec<u16> },
    /// Unrestricted internet (rarely granted).
    All,
}

impl Default for NetworkCapability {
    fn default() -> Self { NetworkCapability::None }
}

/// Resource limits enforced by the sandbox supervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_percent:    f64,
    pub max_memory_bytes:   u64,
    pub max_disk_bytes:     u64,
    pub max_net_bps:        u64,
    pub max_processes:      u32,
    pub timeout_secs:       Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_percent:  50.0,
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB
            max_disk_bytes:   1024 * 1024 * 1024, // 1 GB
            max_net_bps:      10 * 1024 * 1024,   // 10 MB/s
            max_processes:    16,
            timeout_secs:     Some(3600),
        }
    }
}

/// Cryptographically signed capability token issued to each sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Unique sandbox identifier.
    pub sandbox_id:    String,
    /// Component this sandbox is running.
    pub component:     String,
    /// Isolation tier.
    pub tier:          IsolationTier,
    /// File system capabilities.
    pub filesystem:    FilesystemCapability,
    /// Network capabilities.
    pub network:       NetworkCapability,
    /// Set of other sandbox IDs this sandbox may communicate with.
    pub allowed_peers: Vec<String>,
    /// Resource limits.
    pub resources:     ResourceLimits,
    /// Expiry timestamp (nanoseconds since epoch). None = no expiry.
    pub expires_at_ns: Option<u64>,
    /// BLAKE3 signature of all fields above (computed by supervisor).
    pub signature:     String,
}

impl CapabilityToken {
    pub fn new(sandbox_id: String, component: String, tier: IsolationTier) -> Self {
        let mut token = Self {
            sandbox_id,
            component,
            tier,
            filesystem: FilesystemCapability::default(),
            network: NetworkCapability::None,
            allowed_peers: Vec::new(),
            resources: ResourceLimits::default(),
            expires_at_ns: None,
            signature: String::new(),
        };
        token.signature = token.compute_signature();
        token
    }

    /// Compute BLAKE3 signature of all fields (excluding signature itself).
    pub fn compute_signature(&self) -> String {
        let payload = serde_json::json!({
            "sandbox_id": self.sandbox_id,
            "component": self.component,
            "tier": self.tier,
            "filesystem": self.filesystem,
            "network": self.network,
            "allowed_peers": self.allowed_peers,
            "resources": self.resources,
            "expires_at_ns": self.expires_at_ns,
        });
        blake3::hash(payload.to_string().as_bytes()).to_hex().to_string()
    }

    pub fn is_valid(&self) -> bool {
        // Verify signature matches content
        self.signature == self.compute_signature()
        && self.expires_at_ns.map_or(true, |exp| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            exp > now
        })
    }

    pub fn can_read(&self, path: &str) -> bool {
        self.filesystem.read_paths.iter().any(|p| path.starts_with(p.as_str()))
        || (self.filesystem.allow_temp && path.contains("temp"))
    }

    pub fn can_write(&self, path: &str) -> bool {
        self.filesystem.write_paths.iter().any(|p| path.starts_with(p.as_str()))
        || (self.filesystem.allow_temp && path.contains("temp"))
    }

    pub fn can_connect(&self, host: &str, port: u16) -> bool {
        match &self.network {
            NetworkCapability::None => false,
            NetworkCapability::LocalOnly => host == "127.0.0.1" || host == "localhost",
            NetworkCapability::Whitelist { hosts, ports } => {
                hosts.iter().any(|h| h == host) && ports.contains(&port)
            }
            NetworkCapability::All => true,
        }
    }

    pub fn can_talk_to(&self, peer_id: &str) -> bool {
        self.allowed_peers.iter().any(|p| p == peer_id)
    }
}

/// A capability violation recorded for the Survival KB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityViolation {
    pub sandbox_id:        String,
    pub component:         String,
    pub violation_type:    ViolationType,
    pub attempted_action:  String,
    pub blocked:           bool,
    pub timestamp_ns:      u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    FileRead,
    FileWrite,
    NetworkConnect,
    PeerMessage,
    ResourceExceeded,
    SignatureInvalid,
}
