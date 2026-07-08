//! bonsai-sns — Sandbox Nervous System
//!
//! Zero-trust, capability-based isolation for every process, actor, tool,
//! extension, and UI component in the Bonsai Ecosystem.
//!
//! Every component runs in exactly one sandbox. No code runs bare-metal except
//! the SandboxSupervisor itself. Capability tokens are cryptographically signed
//! by the supervisor and enforced at every inter-sandbox communication boundary.
//!
//! # Isolation Tiers
//!
//! | Tier | Technology | Use Cases |
//! |------|-----------|-----------|
//! | Wasm (0) | wasmtime | UI panels, tools, extensions, agent code |
//! | Process (1) | namespaces + seccomp | Trusted daemons, watchdog |
//! | Container (2) | gVisor/runsc | Training scripts, model servers |
//! | MicroVm (3) | Firecracker/KVM | Untrusted extensions, F³ workers |

pub mod capability;
pub mod supervisor;

pub use capability::{
    CapabilityToken, CapabilityViolation, FilesystemCapability,
    IsolationTier, NetworkCapability, ResourceLimits, ViolationType,
};
pub use supervisor::{SandboxInfo, SandboxSupervisor, SandboxStatus, SandboxMessage};

use std::sync::Arc;

/// Create a fully configured SNS supervisor and spawn monitoring tasks.
pub fn start_supervisor() -> Arc<SandboxSupervisor> {
    let sup = SandboxSupervisor::new();
    // Pre-register the core Bonsai components with appropriate tiers
    let core_components = [
        ("model_server",    IsolationTier::Process),
        ("training_script", IsolationTier::Process),
        ("f3_worker",       IsolationTier::Wasm),
        ("swarm_agent",     IsolationTier::Wasm),
        ("extension",       IsolationTier::Wasm),
        ("daemon_main",     IsolationTier::Process),
    ];
    for (name, tier) in &core_components {
        let token = sup.create_token_for(name, *tier);
        sup.register(token);
    }
    sup
}
