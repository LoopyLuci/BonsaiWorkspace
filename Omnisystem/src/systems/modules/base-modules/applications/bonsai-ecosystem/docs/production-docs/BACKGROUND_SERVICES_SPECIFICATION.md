# Next-Generation Background Services & Hybrid Determinism Engine Specification

**Version**: 1.0.0  
**Status**: Production-Ready Architecture Blueprint  
**Last Updated**: 2026-06-05  
**Author**: Omnisystem Architecture Team

---

## Executive Summary

The Omnisystem requires a new class of background services that are:
- **Demand-activated** – Not always running; spawned when needed
- **Snapshotable** – Can pause, serialize state, and resume with full context
- **Deterministic** – Execution can be recorded, replayed, and verified
- **Capability-secured** – Fine-grained access control via unforgeable tokens
- **Self-healing** – Automatic restart from last good snapshot on failure
- **Offline-capable** – Can operate entirely disconnected, then synchronize

Additionally, the system must support optional, safety-clamped AI/ML models that enhance performance and intelligence while maintaining deterministic guarantees.

This specification provides:
1. **Complete architecture** for the Service Lifecycle Manager (SLM)
2. **Integration blueprint** with Bonsai Buddy standalone agent
3. **Hybrid Determinism Engine** design for AI-enhanced optimization
4. **Implementation roadmap** with 8 phases and concrete milestones
5. **Formal verification strategy** using Axiom proofs
6. **Production-grade integration** with existing Omnisystem components

---

## Part 1: Service Lifecycle Manager (SLM)

### 1.1 Core Design Principles

| Principle | Implementation | Rationale |
|-----------|----------------|-----------|
| **Demand-activated** | Services spawned only when requested via capability | Minimize resource waste; enable lazy loading |
| **Snapshotable** | Full state (memory, registers, capabilities) captured as CAS content | Enable pause/resume without data loss |
| **Idempotent restore** | Restoring same snapshot yields identical service state | Enable deterministic debugging and replay |
| **Capability-mediated** | Only clients holding a capability can request service | Fine-grained security; no ambient authority |
| **Deterministic** | Service execution deterministic by design; inputs recorded | Enable formal verification; support replay |
| **Hot-reloadable** | Service binaries updated atomically via UMS without restart | Zero downtime upgrades; atomic version swaps |
| **Resource-controlled** | Each service in Sanctum vault with memory/CPU/I/O limits | Prevent resource starvation; enable SLA enforcement |
| **Self-healing** | Monitor service health; restart from snapshot on failure | Automatic recovery without manual intervention |

### 1.2 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Omnisystem Client Layer                         │
│  (Bonsai Buddy, CLI, services that need background functionality)  │
└─────────────────────────┬───────────────────────────────────────────┘
                          │
                          ▼ (capability-mediated request)
┌─────────────────────────────────────────────────────────────────────┐
│         Service Lifecycle Manager (SLM) – Aether Actor              │
│  • Manages service registry and manifests (UMS integration)        │
│  • Spawns/restores service vaults on demand                        │
│  • Monitors idle timeout and triggers snapshots                    │
│  • Enforces resource quotas and capabilities                       │
│  • Logs all lifecycle events to audit-log                          │
│  • Implements health monitoring with heartbeats                    │
└─────────────────────────┬───────────────────────────────────────────┘
                          │
                          ▼ (spawn vault / restore snapshot)
┌─────────────────────────────────────────────────────────────────────┐
│              Sanctum Vaults (Isolated Service Processes)            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │  net     │  │ storage  │  │  fax     │  │ scanner  │  ...      │
│  │ service  │  │ service  │  │ service  │  │ service  │           │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘           │
│                                                                     │
│  Each vault: isolated memory, restricted capabilities, timed       │
└─────────────────────────┬───────────────────────────────────────────┘
                          │
                          ▼ (snapshot/restore via kernel syscalls)
┌─────────────────────────────────────────────────────────────────────┐
│          Kernel-Level Snapshot Support (UOSC Kernel)               │
│  • snapshot_vault(vault_id) → blake3 hash in CAS                  │
│  • restore_vault(snapshot_hash) → new vault with recovered state  │
│  • Serializes memory, registers, capability table                 │
│  • Compression via Universal Compression Engine (UCE)             │
└─────────────────────────┬───────────────────────────────────────────┘
                          │
                          ▼ (store snapshots, distribute updates)
┌─────────────────────────────────────────────────────────────────────┐
│        CAS + UMS + TransferDaemon (Distributed Storage)            │
│  • Service binaries (UMS modules with manifest)                   │
│  • State snapshots (content-addressed, compressed)                │
│  • Audit log (Universe) – all lifecycle events                    │
│  • Peer-to-peer sync (TransferDaemon for cluster sync)            │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 Service Manifest Schema

Each background service is a UMS module with a manifest declaring its requirements and capabilities:

```yaml
# services/fax/service-manifest.yaml
name: fax
version: 2.0.0
type: service

# Binary reference in CAS
binary: blake3:abcdef1234567890...

# Capabilities this service requires from the SLM
capabilities_required:
  - USB                           # Hardware access
  - NET:outbound                  # Outbound network only
  - Storage:write:/fax/queue      # Specific filesystem path

# Resource limits enforced by kernel
resources:
  memory_mb: 512
  cpu_cores: 2
  cpu_percent_max: 80
  iops_limit: 1000

# Snapshot configuration
snapshotable: true
idle_timeout_secs: 300            # Pause after 5 min inactivity
max_snapshots: 5                  # Keep last 5 snapshots
max_snapshot_size_mb: 256         # Enforce size quota

# Archive configuration
archive_after_hours: 24           # Move to cold storage after 1 day
archive_tier: "network-storage"   # Destination tier

# Health monitoring
heartbeat_interval_secs: 10
heartbeat_timeout_secs: 5

# Service dependencies (must be available before this service starts)
dependencies:
  - name: auth
    version: ">=1.0"

# Signatures (council authority)
signatures:
  - signer: "governance-council-alpha"
    algorithm: "bls"
    signature: "bls-sig-base64..."
```

### 1.4 Service Lifecycle State Machine

```
┌──────────────┐
│   UNSTARTED  │  (Service never spawned or after archival restore)
└──────┬───────┘
       │ (client requests service with capability)
       ▼
┌──────────────┐
│   SPAWNING   │  (Creating vault, loading binary, granting caps)
└──────┬───────┘
       │ (vault ready, initialization complete)
       ▼
┌──────────────┐
│   RUNNING    │  (Service active, accepting requests)
└──────┬───────┘
       │ (idle timeout exceeded)
       ▼
┌──────────────┐
│   PAUSING    │  (on_pause() called, state serialized)
└──────┬───────┘
       │ (snapshot complete)
       ▼
┌──────────────┐
│   PAUSED     │  (Memory snapshot in CAS, vault destroyed)
└──────┬────┬──┘
       │    │
       │    └─► (explicit archive after delay)
       │         └──► ARCHIVED
       │
       └─► (client requests service again)
           └──► RESTORING → RUNNING

┌──────────────┐
│   ARCHIVED   │  (Snapshot moved to cold tier)
└──────┬───────┘
       │ (client requests; fetch from cold tier)
       ▼
    RESTORING
```

### 1.5 Demand-Activated Spawn & Restore Flow

When a client requests a service:

```
Client sends: request_service(service_name, client_capability)
       │
       ▼
SLM receives request
       │
       ├─► Verify client_capability grants access to service_name
       │   (If denied: return CapabilityViolation error)
       │
       ├─► Look up service manifest in UMS registry
       │   (If not found: return ServiceNotFound error)
       │
       └─► Check if snapshot exists
           │
           ├─► YES (snapshot found)
           │   │
           │   ├─► Load snapshot from CAS (or cold tier if archived)
           │   ├─► Create new Sanctum vault
           │   ├─► Restore memory image + capability table
           │   ├─► Call service.on_resume(snapshot_state)
           │   ├─► Record ServiceRestored event in audit-log
           │   └─► Return connection_capability to client
           │
           └─► NO (no snapshot)
               │
               ├─► Create new Sanctum vault
               ├─► Load service binary from UMS
               ├─► Grant required capabilities
               ├─► Call service.initialize()
               ├─► Record ServiceSpawned event in audit-log
               └─► Return connection_capability to client
```

### 1.6 Idle Timeout & Automatic Snapshot

The SLM runs a background idle-watcher loop:

```
loop every 10 seconds:
    for each running service:
        let elapsed = now() - service.last_access
        if elapsed > service.idle_timeout_secs:
            // Trigger pause and snapshot
            service.on_pause()  // Flush buffers, release resources
            let snapshot_hash = kernel.snapshot_vault(vault_id)
            service.snapshot_hash = snapshot_hash
            service.state = PAUSED
            
            // After timeout, consider archiving
            if elapsed > service.archive_after_hours * 3600:
                cas.move_to_tier(snapshot_hash, "cold")
                service.state = ARCHIVED
            
            kernel.destroy_vault(vault_id)
            audit_log.record(ServicePaused { 
                service_id, snapshot_hash, timestamp 
            })
```

### 1.7 Service-Aware Pausing Interface

Every background service must implement the `Snapshotable` trait:

```titan
/// Trait for services that support pause/resume
pub trait Snapshotable {
    /// Called before kernel snapshot
    /// Service must:
    /// 1. Flush any buffered writes to persistent storage
    /// 2. Close transient connections (network, files)
    /// 3. Release resources that cannot be snapshotted
    /// Returns opaque state blob for on_resume
    fn on_pause(&mut self) -> Result<Vec<u8>, Error>;
    
    /// Called after kernel restore
    /// Service must:
    /// 1. Reconstruct internal state from the blob
    /// 2. Re-establish connections if needed
    /// 3. Resume operation from saved point
    fn on_resume(&mut self, state: &[u8]) -> Result<(), Error>;
}
```

This is the only integration point services need. The SLM calls these before/after kernel snapshots.

### 1.8 Resource Control & Quotas

Each service vault has hard limits enforced by the kernel capability system:

```titan
pub struct ResourceQuota {
    // Memory: kernel kills vault if exceeded
    pub memory_limit_mb: u32,
    
    // CPU: kernel scheduler respects cpu_quota capability
    pub cpu_quota_cores: f32,
    pub cpu_percent_max: u32,
    
    // I/O: measured in IOPS
    pub iops_limit: u32,
    
    // Snapshot storage quota
    pub max_snapshots: u32,
    pub max_snapshot_total_size_mb: u32,
}
```

The SLM enforces quotas before spawning:

```titan
pub fn spawn_service(&self, manifest: &ServiceManifest) -> Result<VaultId> {
    // Check SLM-level quotas
    if self.total_memory_allocated + manifest.resources.memory_mb > CLUSTER_MEMORY_LIMIT {
        return Err(QuotaExceeded("Cluster memory limit reached"));
    }
    
    if self.total_snapshots >= manifest.max_snapshots {
        // Delete oldest snapshot
        self.delete_oldest_snapshot(manifest.service_id);
    }
    
    // Kernel enforces resource limits
    let vault_id = kernel::create_vault_with_limits(
        manifest.binary_hash,
        &manifest.capabilities_required,
        &manifest.resources
    )?;
    
    Ok(vault_id)
}
```

### 1.9 Health Monitoring & Self-Healing

The SLM monitors service health and auto-restarts on failure:

```titan
pub async fn health_monitor_loop(&self) {
    loop {
        for each service in self.instances.values() {
            if service.state == ServiceState::RUNNING {
                // Send heartbeat
                let response = timeout(
                    Duration::from_secs(service.heartbeat_timeout_secs),
                    service.send_heartbeat()
                ).await;
                
                match response {
                    Ok(Ok(_)) => {
                        // Service healthy
                        service.consecutive_failures = 0;
                    }
                    _ => {
                        // No response or error
                        service.consecutive_failures += 1;
                        
                        if service.consecutive_failures >= MAX_FAILURES {
                            // Service dead
                            log::error!("Service {} unresponsive, restarting from snapshot", 
                                       service.id);
                            
                            // Destroy failed vault
                            kernel::destroy_vault(service.vault_id);
                            
                            // Restore from last good snapshot
                            service.state = ServiceState::RESTORING;
                            if let Ok(snapshot) = self.get_last_snapshot(service.id) {
                                let vault_id = kernel::restore_vault(&snapshot.hash)?;
                                service.vault_id = vault_id;
                                service.state = ServiceState::RUNNING;
                                service.consecutive_failures = 0;
                            }
                            
                            audit_log.record(ServiceRestartedFromSnapshot {
                                service_id: service.id,
                                snapshot_hash: service.snapshot_hash,
                                reason: "health_check_failed",
                            });
                        }
                    }
                }
            }
        }
        
        sleep(Duration::from_secs(self.health_check_interval)).await;
    }
}
```

### 1.10 Observability & Audit

All lifecycle events are logged to the audit-log (Universe service):

```
ServiceSpawned {
    service_id: u64,
    service_name: String,
    manifest_hash: [u8; 32],
    timestamp: u64,
    requester_capability: Capability,
}

ServiceRestored {
    service_id: u64,
    snapshot_hash: [u8; 32],
    timestamp: u64,
    from_archive: bool,
}

ServicePaused {
    service_id: u64,
    snapshot_hash: [u8; 32],
    timestamp: u64,
    memory_used: u64,
    state_size: u64,
}

ServiceArchived {
    service_id: u64,
    snapshot_hash: [u8; 32],
    archive_tier: String,
    timestamp: u64,
}

ServiceCrashed {
    service_id: u64,
    error: String,
    timestamp: u64,
    last_snapshot: Option<[u8; 32]>,
}

ResourceQuotaExceeded {
    service_id: u64,
    resource_type: String,  // "memory", "cpu", "iops", "snapshots"
    limit: u64,
    actual: u64,
    timestamp: u64,
}
```

The audit-log enables:
- **Time-travel replay**: Reconstruct exact sequence of events
- **Incident forensics**: Understand what led to service failure
- **Compliance audit**: Verify all services operated within quotas
- **Performance analysis**: Identify bottlenecks and optimization opportunities

---

## Part 2: Bonsai Buddy as Standalone Agent

### 2.1 Bonsai Buddy Architecture

Bonsai Buddy is a desktop/mobile companion app (Tauri + native) that can run entirely independently from the Omnisystem cluster.

**Embedded Components**:
```
Bonsai Buddy (Tauri App)
    │
    ├─► SLM Client (Lightweight)
    │   • Capability-mediated requests to services
    │   • Local service spawning (hosted-light adapter)
    │   • Snapshot management
    │
    ├─► Local CAS (Content-Addressed Storage)
    │   • SQLite or LMDB backend
    │   • Stores service binaries and snapshots locally
    │   • Bounded size (e.g., 5 GB max)
    │
    ├─► Local UMS Cache
    │   • Directory of signed service modules
    │   • Synced from cluster when online
    │   • Fallback for offline operation
    │
    ├─► TransferDaemon Node (Optional)
    │   • P2P synchronization when online
    │   • Pushes local changes to cluster
    │   • Pulls remote updates
    │
    └─► User Interface
        • Desktop: Qt/ImGui for rich UI
        • Mobile: Native iOS/Android
```

### 2.2 Operation Modes

**Mode 1: Online (Connected to Omnisystem Cluster)**
- Buddy detects network connection to cluster
- All service requests forwarded to cluster SLM
- Buddy acts as thin client with local caching
- Snapshots synced back to cluster in real-time

**Mode 2: Offline (No Connectivity)**
- Buddy operates entirely locally
- Service requests handled by local SLM
- Services spawned using hosted-light adapter (emulated Sanctum)
- All snapshots stored in local CAS
- User work preserved without external dependency

**Mode 3: Hybrid (Online but Local Processing)**
- Buddy has network but service not available in cluster
- Service runs locally but syncs with cluster when possible
- Useful for computational-heavy operations (image processing, etc.)

### 2.3 Offline-First Architecture

Bonsai Buddy uses event-sourced snapshots with CRDT merging:

```
Local Buddy Instance           Cluster Instance
(Offline)                      (Online)
    │                              │
    ├─► Compose fax               ├─► Fax service snapshot v1
    │   (local snapshot v1)        │
    │                              │
    ├─► Edit document              ├─► Fax service snapshot v2
    │   (local snapshot v2)        │
    │                              │
    ├─► Take offline               ├─► Fax service snapshot v3
    │   (local snapshots synced    │
    │    to local CAS)             │
    │                              │
    │ [Connectivity Restored]       │
    │                              │
    ├─► Push local snapshots ─────►├─► Merge v1, v2, v3
    │   (v1, v2 delta)             │   with cluster snapshots
    │                              │   (CRDT merge)
    │                              │
    │◄─ Pull merged snapshot ───────┤─► Result: unified state
    │   (merged state)             │
    │                              │
    └─► Resume with merged state   │
```

The merge algorithm is deterministic and conflict-free because:
1. Service state is stored as **immutable event log** (event sourcing)
2. Each event has a **causality timestamp** and **originating node ID**
3. Merge applies events in causal order, using CRDT semantics for concurrent edits

### 2.4 Snapshot Synchronization via TransferDaemon

When Bonsai Buddy comes online:

```titan
pub async fn sync_with_cluster(&self, cluster_addr: &str) -> Result<()> {
    // 1. Connect to cluster via TransferDaemon
    let cluster_connection = transfer_daemon.connect(
        cluster_addr,
        self.local_identity
    ).await?;
    
    // 2. Get Merkle tree of local snapshots
    let local_tree = self.local_cas.merkle_tree()?;
    
    // 3. Get Merkle tree of cluster snapshots
    let cluster_tree = cluster_connection.fetch_merkle_tree().await?;
    
    // 4. Compute delta (snapshots in local but not cluster)
    let delta = local_tree.delta(&cluster_tree)?;
    
    // 5. Push delta to cluster
    for (service_id, snapshot_hash) in delta {
        let snapshot_data = self.local_cas.get(&snapshot_hash)?;
        cluster_connection.push_snapshot(service_id, snapshot_data).await?;
    }
    
    // 6. Get cluster's delta (snapshots in cluster but not local)
    let cluster_delta = cluster_tree.delta(&local_tree)?;
    
    // 7. Pull delta from cluster and merge
    for (service_id, snapshot_hash) in cluster_delta {
        let snapshot_data = cluster_connection.pull_snapshot(service_id).await?;
        
        // Merge using CRDT (deterministic, conflict-free)
        let local_snapshot = self.local_cas.get(&snapshot_hash)?;
        let merged = crdt_merge(&local_snapshot, &snapshot_data)?;
        
        // Store merged snapshot
        self.local_cas.put(&service_id, &merged)?;
    }
    
    Ok(())
}
```

---

## Part 3: Hybrid Determinism Engine (HDE)

### 3.1 Core Principle: Deterministic-First, AI-Optional

The Hybrid Determinism Engine extends the deterministic Omnisystem core with optional, safety-clamped AI/ML models that:
- Suggest optimizations (scheduling, prefetching, etc.)
- Are never required – the core always works without AI
- Cannot violate formal safety guarantees
- Can be disabled, updated, or swapped atomically
- Are validated before affecting production

### 3.2 HDE Architecture

```
┌──────────────────────────────────────────────────────────┐
│          Deterministic Core (Always Required)            │
│  • Scheduler (EDF + CFS)                                │
│  • Memory Manager (LRU + swap)                          │
│  • IPC (message passing)                                │
│  • Driver Framework (UDC)                               │
│  • Each subsystem exposes `suggest_*` API               │
└───────────────┬─────────────────────────────────────────┘
                │ (optional advisory request)
                ▼
┌──────────────────────────────────────────────────────────┐
│   AI Advisor Orchestrator (Aether actor)                 │
│  • Loads models from UMS                                │
│  • Runs inference in isolated Sanctum vault              │
│  • Clamps outputs via safety envelopes                  │
│  • Logs suggestions to audit-log                        │
│  • Implements shadow mode for validation                │
└───────────────┬─────────────────────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────────┐
│   Safety Envelope Library (Formally Verified)            │
│  • Per-model clamping functions                         │
│  • Axiom-proved bounds checking                         │
│  • Type-safe I/O schemas                                │
└──────────────────────────────────────────────────────────┘
```

### 3.3 Model Lifecycle

```
1. DATA COLLECTION (Production UVM)
   └─► Record execution traces (kernel logs, metrics)
       • Anonymize sensitive data
       • Store in CAS

2. FEATURE EXTRACTION
   └─► Transform traces into training examples
       • Historical patterns (last 10 accesses, etc.)
       • Current system state (load, memory, etc.)

3. OFFLINE TRAINING
   └─► Small deterministic models (decision trees, linear regression)
       • Fixed random seed (deterministic training)
       • Produces identical weights each run

4. SAFETY ENVELOPE DEFINITION
   └─► Bound model outputs
       • Min/max values
       • Confidence thresholds
       • Resource limits
       • Axiom proof of bounds

5. PACKAGING AS UMS MODULE
   └─► Sign with council key
       • Binary (ONNX, decision tree, etc.)
       • Manifest (I/O schema, envelope bounds)
       • Proof certificate

6. SHADOW MODE DEPLOYMENT
   └─► Run in parallel without affecting production
       • Validate for 10,000+ predictions
       • Measure safety: 0% violations required
       • Measure efficacy: improvement over baseline

7. COUNCIL PROMOTION
   └─► Threshold signature vote
       • Council approves shadow validation results
       • Model activated for advisory use

8. ACTIVE OPERATION
   └─► Subsystems call advisor
       • Clamp suggestion via safety envelope
       • Use if high confidence, ignore otherwise
       • Log all suggestions to audit-log

9. CONTINUOUS MONITORING
   └─► UVM watches for safety violations
       • If violation occurs: immediate demotion to shadow
       • Incident logged

10. HOT-RELOAD UPDATES
    └─► New model published to UMS
        • Orchestrator loads atomically
        • Zero downtime swap
```

### 3.4 Safety Envelopes (Axiom-Proved)

Each model output is bounded by a formally verified safety envelope:

```titan
pub struct SafetyEnvelope {
    // Model must produce values in this range
    pub min_value: f64,
    pub max_value: f64,
    
    // If model's confidence < threshold, return None (use fallback)
    pub min_confidence: f32,
    
    // Inference must complete within this time
    pub max_latency_us: u64,
    
    // Additional bounds specific to the model
    pub custom_bounds: Vec<Bound>,
}

pub fn clamp_output(raw: ModelOutput, envelope: &SafetyEnvelope) -> Option<ModelOutput> {
    // Check confidence threshold
    if raw.confidence < envelope.min_confidence {
        return None;  // Use deterministic fallback
    }
    
    // Clamp value to bounds
    let clamped = raw.value
        .max(envelope.min_value)
        .min(envelope.max_value);
    
    Some(ModelOutput {
        value: clamped,
        confidence: raw.confidence,
    })
}
```

**Axiom Proofs**:
```
theorem envelope_preserves_bounds:
  forall raw: ModelOutput, env: SafetyEnvelope,
    let clamped = clamp_output(raw, env) in
    clamped.is_none() ∨ 
    (clamped.unwrap().value >= env.min_value ∧ 
     clamped.unwrap().value <= env.max_value)

theorem envelope_deterministic:
  forall raw: ModelOutput, env: SafetyEnvelope,
    clamp_output(raw, env) == clamp_output(raw, env)  // idempotent

theorem fallback_guaranteed:
  forall core: DeterministicCore, advisor: Option<AIAdvisor>,
    advisor.is_none() ∨ advisor.unavailable() ⟹ 
    core.decide() completes within timeout
```

### 3.5 Use Cases for HDE

| Subsystem | Deterministic Core | AI Enhancement | Safety Envelope |
|-----------|-------------------|-----------------|-----------------|
| **Scheduler** | CFS + EDF | Predicts load; suggests CPU affinity | Never pre-empt real-time; CPU share ∈ [1%, 100%] |
| **Memory** | LRU + swap | Predicts next page access | Pre-fetch within 2× read-ahead |
| **Storage** | Fixed read-ahead | Predicts streaming patterns | Block within file bounds |
| **Network** | CUBIC + BBR | Predicts congestion | cwnd ∈ [2 MSS, BDP] |
| **Aether** | Least-loaded | Suggests optimal node | Node must be online & have capacity |
| **Power** | On-demand governor | Predicts load; suggests frequency | Respect thermal limits, battery reserve |
| **UDC** | Rule-based matching | Suggests similar device mappings | Confidence > 0.95, type check passes |

### 3.6 Integration Pattern: Arbiter

Each subsystem uses the arbiter pattern to decide whether to use AI suggestions:

```titan
fn schedule_next_task(queues: &[TaskQueue]) -> Task {
    // 1. Try AI advisor (non-blocking, timeout 100µs)
    let suggestion = match ai_advisor.suggest_next_task(queues) {
        Some(s) if s.confidence >= MIN_CONFIDENCE => Some(s),
        _ => None,
    };
    
    // 2. Apply safety envelope
    let clamped = suggestion.and_then(|s| {
        safety_envelope::clamp_task(s.task, queues)
    });
    
    // 3. Validate
    if let Some(task) = clamped {
        if is_valid_task(&task) {
            return task;  // Use AI suggestion
        }
    }
    
    // 4. Fallback to deterministic core (always works)
    deterministic_cfs_schedule(queues)
}
```

The AI advisor call is non-blocking. If it times out or the advisor crashes, the deterministic path is taken immediately.

---

## Part 4: Implementation Roadmap

### Phase 1: Kernel Extensions (2-3 weeks)
**Objective**: Enable single-vault snapshots

**Deliverables**:
- [ ] `snapshot_vault(vault_id)` syscall
- [ ] `restore_vault(snapshot_hash)` syscall
- [ ] UCE integration for snapshot compression
- [ ] CAS storage backend for snapshots
- [ ] Unit tests (vault snapshot/restore cycle)

**Success Criteria**:
- Snapshot creates identical memory image
- Restore resumes execution from saved point
- Snapshot size < 10% of original memory
- Latency: snapshot < 100ms, restore < 500ms

---

### Phase 2: SLM Actor Implementation (3-4 weeks)
**Objective**: Build Service Lifecycle Manager in Aether

**Deliverables**:
- [ ] SLM actor with service registry
- [ ] Spawn/restore state machine
- [ ] Idle timeout monitoring loop
- [ ] Health heartbeat loop
- [ ] Resource quota enforcement
- [ ] Audit-log integration
- [ ] 50+ unit tests

**Success Criteria**:
- Service spawn: < 1 second
- Service restore from snapshot: < 500ms
- Idle timeout triggers correctly
- Health monitoring detects failures within 5 seconds
- All tests pass

---

### Phase 3: Service Manifest & UMS Integration (1-2 weeks)
**Objective**: Define service manifest schema and wire UMS

**Deliverables**:
- [ ] Service manifest schema (YAML/TOML)
- [ ] UMS manifest validation
- [ ] Service registry querying
- [ ] Manifest signing (council key)
- [ ] 20+ unit tests

**Success Criteria**:
- Manifest validation catches all errors
- Service discovery works
- Signature verification passes

---

### Phase 4: Snapshotable SDK (1-2 weeks)
**Objective**: Provide library for service developers

**Deliverables**:
- [ ] `Snapshotable` trait (Titan)
- [ ] Serialization helpers (term heap)
- [ ] Example implementations (file server, network proxy)
- [ ] Documentation (100+ lines per example)
- [ ] 30+ unit tests

**Success Criteria**:
- Examples compile and run
- Snapshot/resume cycle preserves state
- Examples in docs compile cleanly

---

### Phase 5: Bonsai Buddy Integration (2-3 weeks)
**Objective**: Embed SLM client + local CAS in Tauri app

**Deliverables**:
- [ ] SLM client (Titan → Rust C FFI)
- [ ] Local CAS backend (SQLite)
- [ ] Local UMS cache (filesystem)
- [ ] Online/offline mode detection
- [ ] Snapshot sync logic (TransferDaemon)
- [ ] 30+ unit tests
- [ ] 3+ integration tests (offline compose → online send)

**Success Criteria**:
- Offline mode spawns services locally
- Online mode syncs with cluster
- User work preserved across offline/online transitions
- No data loss in any scenario

---

### Phase 6: AI Advisor Orchestrator (3-4 weeks)
**Objective**: Implement HDE core

**Deliverables**:
- [ ] Orchestrator actor (Aether)
- [ ] Model loading and caching
- [ ] Shadow mode support
- [ ] Safety envelope library (Titan)
- [ ] Hot-reload logic
- [ ] 50+ unit tests

**Success Criteria**:
- Models load from UMS
- Shadow mode validates for 10,000 predictions
- Safety envelope clamping works
- Hot-reload zero downtime

---

### Phase 7: Model Building Framework (3-4 weeks)
**Objective**: Build trace-to-model pipeline

**Deliverables**:
- [ ] Trace collection from UVM
- [ ] Feature extraction (Sylva)
- [ ] Training pipeline (decision trees, linear regression)
- [ ] Model packaging as UMS module
- [ ] Axiom proof generation
- [ ] 30+ unit tests

**Success Criteria**:
- Training deterministic (same seed → same model)
- Models < 1 MB
- Inference < 1ms latency

---

### Phase 8: Formal Verification (2-3 weeks)
**Objective**: Axiom proofs for HDE and SLM

**Deliverables**:
- [ ] Axiom proofs for safety envelopes
- [ ] SLM invariants (no snapshot loss, restore idempotency)
- [ ] HDE proofs (fallback guaranteed, no deadlock)
- [ ] CI integration (proofs block regression)

**Success Criteria**:
- All proofs verified by Axiom
- CI blocks any regression
- Proof documentation clear

---

### Integration Roadmap Summary

| Phase | Component | Duration | Dependencies | Risk |
|-------|-----------|----------|--------------|------|
| 1 | Kernel extensions | 2-3 wks | None | Low |
| 2 | SLM Actor | 3-4 wks | Phase 1 | Low |
| 3 | UMS integration | 1-2 wks | Phase 2 | Very Low |
| 4 | SDK | 1-2 wks | Phase 3 | Very Low |
| 5 | Buddy integration | 2-3 wks | Phases 2-4 | Medium |
| 6 | HDE Orchestrator | 3-4 wks | Phases 1-3 | Medium |
| 7 | Model framework | 3-4 wks | Phase 6 | Medium |
| 8 | Formal verification | 2-3 wks | All | Low |

**Critical Path**: Phase 1 → 2 → 3 → 5 (SLM core), parallel with 6 → 7 (HDE core)

**Total Estimated Duration**: 16-24 weeks (4-6 months)

---

## Part 5: Integration with Omnisystem Components

### 5.1 Integration Points

| Component | Role in Design | Existing | Changes |
|-----------|----------------|----------|---------|
| **UOSC Kernel** | Provides snapshot/restore syscalls | Partial (memfork) | Extend for single-vault snapshots |
| **Sanctum Vaults** | Service isolation | ✓ Full | Use as-is for service spawning |
| **UMS** | Service binary & manifest storage | ✓ Full | Add `service` module type |
| **CAS** | Snapshot content-addressed storage | ✓ Full | Already used |
| **TransferDaemon** | P2P snapshot sync | ✓ Full | Already used |
| **Aether Actors** | SLM and HDE Orchestrator | ✓ Full | Use for both |
| **UVM** | Shadow mode validation, chaos testing | ✓ Full | Extend for HDE validation |
| **Audit-log** | Lifecycle event logging | ✓ Full | Already used |
| **Capability System** | Service access control | ✓ Full | Already used |

**No new components required** – the design re-organizes existing capabilities.

### 5.2 UMS Service Module Type

Extend UMS to support `service` module type:

```yaml
# UMS Registry Entry
module:
  name: fax
  version: 2.0.0
  type: service          # NEW: service module type
  
  # Standard UMS fields
  content:
    binary_hash: blake3:...
    manifest_hash: blake3:...
  
  signatures:
    - authority: "governance-council"
      algorithm: "bls"
      value: "..."
  
  # Service-specific metadata
  service_manifest:
    idle_timeout_secs: 300
    max_snapshots: 5
    max_snapshot_size_mb: 256
    capabilities_required: [USB, NET:outbound]
    resources:
      memory_mb: 512
      cpu_cores: 2
```

The SLM queries UMS for modules of type `service`.

### 5.3 Capability System Integration

Service access controlled via capabilities:

```titan
// Client must hold this capability to request service
pub struct ServiceCapability {
    service_name: String,
    service_id: u64,
    access_mode: AccessMode,  // read, write, execute
    valid_until: Timestamp,
}

// SLM checks: client.capabilities contains capability for requested_service
if !client_caps.contains(ServiceCapability { 
    service_name: "fax", 
    .. 
}) {
    return Err(CapabilityViolation("No fax capability"));
}
```

This ensures fine-grained, unforgeable permission tokens.

### 5.4 Aether Actor Communication

SLM is a privileged Aether actor that services send heartbeat messages to:

```aether
actor ServiceLifecycleManager {
    // Handles service requests
    on request_service(name: String, cap: Capability) -> ConnectionCap { ... }
    
    // Handles service heartbeat responses
    on heartbeat_response(service_id: u64, status: HealthStatus) { ... }
    
    // Handles UMS module updates (model hot-reload)
    on module_updated(module_id: String, new_hash: Blake3) { ... }
}
```

Services and the HDE Orchestrator communicate with SLM via Aether message passing.

---

## Part 6: Formal Verification Strategy

### 6.1 Axiom Proof Goals

**SLM Invariants**:
- `no_snapshot_loss`: After snapshot taken, never deleted before archived or newer snapshot confirmed
- `restore_idempotent`: Restore(snapshot_hash) applied twice yields identical state
- `capability_propagation`: Service never granted capability client doesn't possess
- `deadlock_free`: SLM state machine never deadlocks

**HDE Invariants**:
- `envelope_preserves_bounds`: Clamped output always within bounds
- `envelope_deterministic`: Same input → same clamped output
- `fallback_guaranteed`: If advisor unavailable, core continues within timeout
- `no_model_deadlock`: Model inference cannot block core
- `shadow_isolation`: Shadow model outputs never influence production decisions
- `hot_reload_atomicity`: Model swap atomic; no request sees partial model

### 6.2 Proof Structure

```axiom
// kernel/snapshot.ax
module Snapshot {
    // Define snapshot_vault and restore_vault semantics
    def snapshot_vault(vault_id) =
        let state = vault_memory[vault_id] in
        let hash = BLAKE3(compress(state)) in
        (CAS.put(hash, compress(state)), hash)
    
    def restore_vault(hash) =
        let compressed = CAS.get(hash) in
        let state = decompress(compressed) in
        allocate_vault(state)
    
    // Prove restore idempotency
    theorem restore_idempotent:
        forall vault_id, hash,
            let v1 = restore_vault(hash) in
            let v2 = restore_vault(hash) in
            vault_memory[v1] == vault_memory[v2]
}

// hde/safety_envelope.ax
module SafetyEnvelope {
    def clamp_output(raw, envelope) =
        if raw.confidence < envelope.min_confidence then None
        else Some({
            value = max(envelope.min, min(raw.value, envelope.max)),
            confidence = raw.confidence
        })
    
    theorem envelope_preserves_bounds:
        forall raw, envelope,
            let clamped = clamp_output(raw, envelope) in
            clamped.is_none() ∨ 
            (clamped.value >= envelope.min ∧ 
             clamped.value <= envelope.max)
}
```

### 6.3 CI Integration

Proofs run as part of CI pipeline:

```bash
# ci/verify.sh
set -e

# Build all proofs
axiom verify kernel/snapshot.ax
axiom verify services/slm.ax
axiom verify hde/safety_envelope.ax
axiom verify hde/determinism.ax

echo "✓ All Axiom proofs verified"

# Run tests
cargo test --all

echo "✓ All tests passed"
echo "✓ Build ready for merge"
```

Proof regression blocks merge – no proof failures allowed.

---

## Part 7: Testing Strategy

### 7.1 Unit Tests (Phase-by-Phase)

Each phase includes comprehensive unit tests:

**Phase 1 (Kernel)**:
- Snapshot creation produces valid content-addressed hash
- Restore recovers exact memory image
- Compression reduces size by >90%
- Latency within bounds

**Phase 2 (SLM)**:
- Service spawn creates vault
- Service restore recovers state
- Idle timeout triggers
- Health check detects failures
- Resource quota enforced

**Phase 4 (SDK)**:
- Services compile with Snapshotable trait
- on_pause() serializes state
- on_resume() deserializes state
- Round-trip preserves state exactly

**Phase 5 (Buddy)**:
- Offline spawn works
- Online sync merges snapshots
- State preserved across offline/online

**Phase 6 (HDE)**:
- Model loads and unloads
- Shadow mode doesn't affect production
- Safety envelope clamps correctly
- Hot-reload atomic

**Phase 7 (Model Framework)**:
- Trace collection deterministic
- Training produces deterministic weights
- Model packaging signs correctly

### 7.2 Integration Tests

**SLM Integration**:
```
Test 1: Spawn → Use → Idle → Pause → Snapshot → Destroy → Restore → Use
Expected: Service resumes exactly where paused

Test 2: Spawn → Crash → Auto-Restart from Snapshot
Expected: Service recovers without manual intervention

Test 3: Multiple services (10+) with different timeouts
Expected: Each paused/restored independently, no interference
```

**Buddy Integration**:
```
Test 1: Compose fax offline → Go online → Send fax
Expected: Work preserved, sent successfully

Test 2: Two Buddies offline → Both go online → Sync
Expected: Snapshots merged correctly, no data loss

Test 3: Network interrupted mid-sync → Retry
Expected: Sync completes, no corruption
```

**HDE Integration**:
```
Test 1: Shadow model → Accumulate 10k predictions → Promote
Expected: Zero safety violations, model promoted safely

Test 2: Active model → Violation detected → Demotion
Expected: Immediate demotion, deterministic fallback used

Test 3: Hot-reload new model → Atomic swap
Expected: Zero requests dropped, no visible latency
```

### 7.3 Chaos Testing (UVM)

The UVM runs continuous chaos tests:

```
Chaos Test 1: Random service crashes
Expected: SLM recovers all services from snapshots

Chaos Test 2: Snapshot corruption
Expected: CAS detects corruption (hash mismatch)

Chaos Test 3: Network partitions during sync
Expected: Buddy retries and completes sync

Chaos Test 4: Model timeouts
Expected: Deterministic fallback used, no hangs

Chaos Test 5: Memory pressure
Expected: Oldest snapshots archived automatically
```

---

## Part 8: Production Deployment Considerations

### 8.1 Graduation Criteria

**Phase 1 (Kernel)**: Enabled behind feature flag, tested on dev clusters only

**Phase 2 (SLM)**: Alpha production deployment, monitor for 1 month, then GA

**Phase 3-4 (Core Services)**: General availability, enabled by default

**Phase 5 (Buddy)**: Beta release, optional for users

**Phase 6-7 (HDE)**: Shadow mode default, models require explicit promotion

**Phase 8 (Verification)**: All proofs integrated into CI

### 8.2 Configuration Parameters

```titan
// SLM Configuration
pub struct SLMConfig {
    // Idle timeout before pausing service
    idle_timeout_secs: u32,  // default: 300
    
    // Archive timeout after idle
    archive_after_hours: u32,  // default: 24
    
    // Health check interval
    health_check_interval_secs: u32,  // default: 10
    
    // Consecutive failures before restart
    max_consecutive_failures: u32,  // default: 3
    
    // Maximum services per SLM instance
    max_services: u32,  // default: 10000
    
    // Total memory quota for all services
    total_memory_quota_mb: u32,  // default: 100000
}

// HDE Configuration
pub struct HDEConfig {
    // Enable AI advisor
    enable_ai_advisor: bool,  // default: true
    
    // Shadow mode minimum samples before promotion
    shadow_mode_min_samples: u32,  // default: 10000
    
    // Model inference timeout
    inference_timeout_us: u64,  // default: 1000
    
    // Enable specific advisors
    advisor_scheduler: bool,  // default: true
    advisor_memory: bool,  // default: true
    advisor_network: bool,  // default: false (experimental)
}
```

---

## Conclusion

This specification provides a complete blueprint for implementing production-grade background services and a hybrid determinism engine. The design:

✅ **Maintains determinism as foundation** – AI is optional, core always works  
✅ **Enables offline-first operation** – Bonsai Buddy works disconnected  
✅ **Provides automatic recovery** – Snapshot-based self-healing  
✅ **Ensures formal correctness** – Axiom proofs throughout  
✅ **Supports atomic updates** – Hot-reload without downtime  
✅ **Integrates seamlessly** – Uses existing Omnisystem components  

The implementation roadmap is concrete, testable, and achievable in 4-6 months. All phases are properly scoped with clear success criteria and risk mitigation strategies.

---

**Next Steps**:
1. ✓ Review specification for gaps
2. Execute Phase 1 (kernel extensions)
3. Proceed through phases in dependency order
4. Run CI/chaos tests continuously
5. Gate promotions on formal verification

