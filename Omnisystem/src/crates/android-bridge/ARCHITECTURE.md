# Android Bridge Architecture & Design Document

## Executive Summary

The Bonsai Android Bridge is a production-grade system for discovering, connecting to, and controlling Android devices at scale. It implements a zero-trust security model with capability-based access control, high-performance screen streaming, and comprehensive observability.

**Key Characteristics:**
- **Zero-Trust**: Every operation requires explicit capability tokens
- **Encryption-First**: Noise protocol + AES-256-GCM on all channels
- **Observable**: W&B + Bonsai Universe event logging
- **Scalable**: 1-1000+ devices per bridge instance
- **Fast**: <50ms screen latency, <30ms input latency

## System Architecture

### Multi-Layer Design

```
Layer 1: User Interface (Tauri)
├─ Android Panel Component (Svelte)
├─ Device List & Status
├─ Screen Mirror Display
├─ Input Control Panel
└─ File Browser

Layer 2: API Layer (Tauri Commands & MCP)
├─ Device Management Commands
├─ Screen Streaming Control
├─ Input Injection API
├─ File Sync Commands
└─ Capability Management

Layer 3: Bridge Logic (Rust Core)
├─ AndroidBridge (Main Orchestrator)
├─ DevicePool (Concurrent Device Management)
├─ CapabilityRegistry (Zero-Trust)
├─ TelemetryCollector (Observability)
└─ Various Subsystems

Layer 4: Transport (TransferDaemon)
├─ Noise Protocol Handshake
├─ Session Management
├─ End-to-End Encryption
└─ Connection Pooling

Layer 5: Network (TCP/WebSocket)
└─ Direct connection to Android Agent

Layer 6: Android Device
├─ BonsaiAgent Service
├─ Screen Encoder (H.264/H.265)
├─ Input Handler
├─ File Sync Service
├─ App Manager
└─ Sensor Provider
```

## Detailed Module Design

### 1. Connection Module (`connection.rs`)

**Responsibilities:**
- Orchestrate all bridge operations
- Manage device lifecycle
- Enforce capability-based access control
- Provide high-level API

**Key Components:**

```rust
pub struct AndroidBridge {
    device_pool: Arc<DevicePool>,           // Active connections
    discovery: Arc<DiscoveryService>,       // mDNS + scanning
    registry: Arc<ManualDeviceRegistry>,    // Persistent device list
    capability_registry: Arc<CapabilityRegistry>,  // Access control
    telemetry: Arc<TelemetryCollector>,    // Event logging
    identity: DeviceIdentity,               // Bridge PKI
}

pub struct ConnectionHandle {
    device_id: String,
    device: Device,
    bridge: Arc<AndroidBridge>,
}
```

**Methods & Flows:**

```
initialize()
├─ Start discovery service
├─ Create identity
├─ Emit initialization event
└─ Ready for connections

register_device(id, name, model, api_level, ip, port, public_key)
├─ Validate input
├─ Store in registry
├─ Emit DeviceDiscovered event
└─ Device becomes available

connect(device_id)
├─ Look up device in registry
├─ Create Device instance
├─ Add to device_pool
├─ Emit Connected event
└─ Return ConnectionHandle

disconnect(device_id)
├─ Look up device
├─ Clear connection state
├─ Emit Disconnected event
└─ Remove from pool

issue_capability(device_id, subject, capability, hours)
├─ Create CapabilityToken
├─ Sign with bridge identity
├─ Store in capability_registry
├─ Emit CapabilityGranted event
└─ Return token ID

revoke_capability(token_id)
├─ Look up token
├─ Mark as revoked
├─ Emit CapabilityRevoked event
└─ Propagate revocation
```

**Concurrency Model:**
- RwLock on device pool (readers for most ops, writers for mutations)
- Arc for shared ownership
- No global locks - permits scale to 1000+ devices

### 2. Discovery Module (`discovery.rs`)

**Two-Level Discovery Strategy:**

```
Level 1: mDNS Broadcast
├─ Discover _bonsai-android._tcp.local.
├─ Parse TXT records for device info
└─ Auto-update device list (5-minute TTL)

Level 2: Manual Registry
├─ User-configured devices
├─ Fallback when mDNS unavailable
├─ Persistent across restarts
└─ Network-specific configs
```

**DiscoveryService Implementation:**

```rust
pub struct DiscoveryService {
    devices: Arc<RwLock<Vec<DiscoveredDevice>>>,
    discovery_interval: Duration,
}

// Runs background task every N seconds:
async fn discovery_loop() {
    loop {
        match mdns_query().await {
            Ok(devices) => {
                // Update device list
                // Remove stale entries (>5 min no heartbeat)
            }
            Err(e) => {
                // Log error, continue
            }
        }
        sleep(discovery_interval).await;
    }
}
```

**Device Metadata (TXT Records):**
```
name=Pixel 6
model=Pixel 6
api_level=31
port=5037
public_key=<ed25519-pk-hex>
```

**ManualDeviceRegistry:**
```rust
register(id, name, model, api_level, ip, port, public_key)
unregister(device_id)
get_device(device_id)
get_devices()
update_last_seen(device_id)
```

### 3. Capability Module (`capability.rs`)

**Zero-Trust Access Control**

Every action protected by capabilities:
- ScreenStream - view device screen
- InputInjection - send touch/keyboard
- FileRead - read device files
- FileWrite - write device files
- AppDeploy - install APKs
- SensorAccess - read sensors
- ShellExecution - run commands

**CapabilityToken Structure:**

```rust
pub struct CapabilityToken {
    id: Uuid,                           // Unique token ID
    capability: CapabilityType,         // What this grants
    device_id: String,                  // Bound to device
    subject: String,                    // Issued to agent/user
    issued_at: DateTime<Utc>,          // Creation time
    expires_at: DateTime<Utc>,         // TTL
    revoked: bool,                      // Revocation flag
    scope: Option<String>,              // Context (e.g., "/Downloads")
    signature: Vec<u8>,                 // Ed25519 sig
    signing_key: Option<SigningKey>,   // Private key (issuer only)
}
```

**Token Lifecycle:**

```
1. Issue
   ├─ Create token with expiry
   ├─ Sign with bridge's Ed25519 key
   ├─ Store in capability_registry
   ├─ Emit CapabilityGranted event
   └─ Return token ID to caller

2. Use
   ├─ Caller submits token
   ├─ Bridge verifies signature
   ├─ Check device matches
   ├─ Check subject matches
   ├─ Check not expired
   ├─ Check not revoked
   └─ Grant access if all checks pass

3. Revoke
   ├─ Issuer calls revoke_token(id)
   ├─ Mark token as revoked
   ├─ Emit CapabilityRevoked event
   └─ Subsequent uses denied
```

**CapabilityRegistry:**

```rust
pub struct CapabilityRegistry {
    tokens: RwLock<HashMap<Uuid, CapabilityToken>>,
    revoked_ids: RwLock<HashSet<Uuid>>,
}

issue_token(token)          // Add to registry
has_capability(device, subject, capability)  // Check if allowed
revoke_token(token_id)      // Revoke by ID
revoke_device_tokens(device_id)  // Revoke all for device
get_device_tokens(device_id)     // Retrieve all for device
```

**Security Properties:**
- Time-bounded (no permanent access)
- Scope-aware (can limit to resources)
- Signature-verified (can't forge)
- Revocable (instant propagation)
- Audit-logged (all issued/revoked)

### 4. Security Module (`security.rs`)

**Noise Protocol Implementation**

Provides encrypted transport layer:

```
Pattern: IK (Initiator has static pubkey, Responder known)

Message 1 (Desktop → Android):
├─ e (ephemeral key)
└─ es (encryption with ephemeral)

Message 2 (Android → Desktop):
├─ e (ephemeral key)
├─ ee (DH on both ephemerals)
├─ se (DH of Android static with Desktop ephemeral)
└─ payload (signed Android identity)

Message 3 (Desktop → Android):
├─ se (DH of Desktop static with Android ephemeral)
├─ payload (signed Desktop identity, capabilities)
└─ Transport state: both directions ready
```

**DeviceIdentity:**

```rust
pub struct DeviceIdentity {
    fingerprint: String,                // Public key hash
    secret_key: Arc<Mutex<StaticSecret>>,  // X25519 static
    public_key: PublicKey,              // Derived public key
}

// Each device/bridge has unique identity
// Fingerprint = blake3(public_key)[0:16] in hex
```

**SessionKey:**

```rust
pub struct SessionKey {
    key: [u8; 32],
    nonce_counter: Arc<Mutex<u64>>,
}

// AES-256-GCM symmetric encryption
encrypt(plaintext) -> ciphertext
decrypt(ciphertext) -> plaintext

// Nonce = counter as LE u64 + padding
// Prevents replay attacks
```

**Cryptographic Guarantees:**

| Property | Mechanism | Notes |
|----------|-----------|-------|
| Confidentiality | AES-256-GCM | 256-bit keys |
| Integrity | GCM authentication | 128-bit tags |
| Authenticity | Ed25519 | During handshake |
| Forward Secrecy | Ephemeral DH | Per-session keys |
| Replay Prevention | Counter + nonce | 64-bit counter |

### 5. Device Module (`device.rs`)

**Device State Management**

```rust
pub enum DeviceStatus {
    Discovered,      // Found but not connected
    Connecting,      // Connection in progress
    Connected,       // Connected and authenticated
    Pairing,         // Capability negotiation
    Paired,          // Ready for operations
    Disconnected,    // Connection lost
    Error,           // Error state
}

pub struct Device {
    id: String,
    name: String,
    model: String,
    api_level: u32,
    status: DeviceStatus,
    ip: String,
    port: u16,
    identity: Arc<DeviceIdentity>,
    capabilities: Vec<CapabilityType>,
    connected_at: Option<DateTime<Utc>>,
    last_heartbeat: DateTime<Utc>,
    metrics: DeviceMetrics,
}
```

**DevicePool:**

```rust
pub struct DevicePool {
    devices: Arc<RwLock<HashMap<String, Device>>>,
}

// Read operations (cheap):
├─ get_device(id)
├─ get_all_devices()
├─ get_devices_by_status(status)
├─ has_device(id)
└─ device_count()

// Write operations (exclusive):
├─ add_device(device)
├─ remove_device(id)
└─ update_device(id, updater)
```

**Heartbeat & Health:**

```
Every 10 seconds:
├─ Send heartbeat ping to device
├─ Update last_heartbeat timestamp
└─ Mark device responsive if reply within 5s

Device is "responsive" if:
└─ last_heartbeat within 30 seconds
```

**Metrics:**

```rust
pub struct DeviceMetrics {
    screen_frames_sent: u64,
    input_events_processed: u64,
    files_synced: u64,
    avg_screen_latency: f64,
    total_data_transferred: u64,
    connection_uptime: u64,
    last_error: Option<String>,
    battery_level: Option<u8>,
    device_temperature: Option<f32>,
}
```

### 6. Streaming Module (`streaming.rs`)

**Architecture:**

```
Device (H.264/H.265 encoder)
    ↓ H264Frame
TransferDaemon (encrypted channel)
    ↓ AES-256-GCM encrypted
ScreenStreamer (frame queue + metrics)
    ↓ UnboundedChannel
UI Thread (Svelte component)
    ↓ WebGL rendering
User Screen
```

**ScreenFrame:**

```rust
pub struct ScreenFrame {
    timestamp_us: u64,          // Device time
    sequence: u64,              // Frame number for reordering
    frame_data: Vec<u8>,        // Encoded data
    is_keyframe: bool,          // I-frame marker
    width: u32,                 // Resolution
    height: u32,
    fps: u8,                    // Framerate hint
    codec: VideoCodec,          // H264 or H265
    checksum: u32,              // CRC32
}
```

**Adaptive Bitrate Algorithm:**

```
Monitor network metrics every 1 second:
├─ estimated_bandwidth_kbps
├─ packet_loss_rate
├─ rtt_ms
└─ jitter_ms

Adjust target bitrate:
├─ If bandwidth < 2000 kbps:
│  └─ Reduce bitrate 20% (min 1000 kbps)
├─ If bandwidth > 10000 kbps:
│  └─ Increase bitrate 20% (max 20000 kbps)
└─ Otherwise: maintain current

Target is hit via device-side encoder settings
```

**Performance Targets:**

| Component | Target | Mechanism |
|-----------|--------|-----------|
| Capture latency | <10ms | Hardware encoder |
| Transport latency | <20ms | Gigabit LAN |
| Decode latency | <10ms | GPU accelerated |
| Display latency | <10ms | Vsync |
| **Total** | **<50ms** | All in parallel |

### 7. Input Module (`input.rs`)

**Event Types:**

```
Touch Events:
├─ DOWN (pointer contact)
├─ MOVE (pointer motion)
├─ UP (pointer release)
└─ CANCEL (gesture aborted)

Keyboard Events:
├─ PRESS (key down)
└─ RELEASE (key up)
└─ plus 256 Android key codes

Pointer Events:
├─ Button press/release
├─ Scroll wheel
└─ For mouse/trackpad
```

**InputEvent Structure:**

```rust
pub struct InputEvent {
    timestamp_us: u64,          // When generated
    id: u32,                    // For acking
    event_type: InputEventType, // Touch/Key/Pointer
    
    // Event-specific data
    touch: Option<TouchEvent>,      // x, y, action, pressure
    keyboard: Option<KeyboardEvent>, // key_code, modifiers, repeat
    pointer: Option<PointerEvent>,   // x, y, button, scroll
}
```

**Gesture Synthesis:**

```
Swipe(x1, y1, x2, y2, duration_ms):
├─ touch_down(x1, y1)
├─ For each of N interpolated points:
│  └─ touch_move(x_i, y_i)
└─ touch_up(x2, y2)

LongPress(x, y, duration_ms):
├─ touch_down(x, y)
├─ sleep(duration_ms)
└─ touch_up(x, y)

PinchZoom(cx, cy, start_radius, end_radius, duration_ms):
├─ For each frame:
│  ├─ touch_move(finger1, at_radius)
│  └─ touch_move(finger2, at_radius)
└─ touch_up on both fingers
```

**InputInjector:**

```rust
pub struct InputInjector {
    event_queue: mpsc::UnboundedSender<InputEvent>,
    event_counter: Arc<Mutex<u32>>,
}

// High-level API:
├─ touch_down(x, y, pointer_id)
├─ touch_move(x, y, pointer_id)
├─ touch_up(x, y, pointer_id)
├─ key_press(key_code)
├─ key_release(key_code)
├─ inject_text(string)
├─ click(x, y)
└─ swipe(x1, y1, x2, y2, duration_ms)

// Low-level API:
└─ inject(InputEvent)
```

### 8. File Sync Module (`file_sync.rs`)

**Content-Addressed Blob Model:**

```
Source File
    ↓ read
SHA256/BLAKE3 hash
    ↓
BlobRef { hash, size, compression }
    ↓ stored in
CAS (Content-Addressable Store)
    ↓
Recipient retrieves by hash
    ↓
Reconstructs file
```

**FileSyncOp:**

```rust
pub struct FileSyncOp {
    id: String,                 // Operation ID
    event_type: FileSyncEventType,  // Create/Modify/Delete/Rename
    metadata: FileMetadata,     // Path, size, timestamps
    direction: SyncDirection,   // Push/Pull/Bidirectional
    blob_ref: Option<BlobRef>,  // BLAKE3 hash reference
    delta_blocks: Option<Vec<DeltaBlock>>,  // Incremental updates
    device_id: String,
}
```

**Delta Sync Algorithm:**

```
OLD FILE: hash_old
NEW FILE: hash_new

If hash_old == hash_new:
└─ No-op

If hash_old != hash_new:
├─ Compute rolling hash (weak collision detection)
├─ Identify changed blocks
├─ For each changed block:
│  ├─ Compress with zstd/brotli
│  └─ Queue DeltaBlock(offset, length, data)
└─ Send only changed blocks (90% reduction for small changes)
```

**FileSynchronizer:**

```rust
pub struct FileSynchronizer {
    sync_root: PathBuf,
    metadata_cache: Arc<RwLock<HashMap<String, FileMetadata>>>,
    pending_ops: mpsc::UnboundedSender<FileSyncOp>,
}

// Operations:
├─ scan_directory() -> Vec<FileMetadata>
├─ detect_changes(device_id) -> async
├─ apply_sync_op(op) -> async
└─ get_status() -> SyncStatus
```

**Change Detection:**

```
1. Scan filesystem:
   ├─ For each file:
   │  ├─ Read metadata (size, mtime)
   │  └─ Hash content (BLAKE3)
   └─ Store in metadata_cache

2. Compare with previous:
   ├─ File in cache but not on disk → Deleted
   ├─ File on disk but not in cache → Created
   ├─ File in both but hash differs → Modified
   └─ Hash unchanged → No sync needed

3. Queue operations:
   ├─ For each change:
   │  ├─ Create FileSyncOp
   │  ├─ Add blob_ref or delta_blocks
   │  └─ Send to pending_ops queue
   └─ Device receives and applies
```

### 9. Telemetry Module (`telemetry.rs`)

**Event Pipeline:**

```
Event Creation (in any module)
    ↓
TelemetryCollector.record()
    ├─ Send to external collector (async, non-blocking)
    ├─ Buffer in memory (circular buffer, max 1000 events)
    └─ Return immediately (fire-and-forget)

External Collectors (in parallel):
├─ W&B Integration
│  └─ HTTP POST to weights & biases API
├─ Bonsai Universe Bridge
│  └─ Send event to universe event bus
└─ Local logging
   └─ Write to structured log
```

**TelemetryEvent:**

```rust
pub struct TelemetryEvent {
    id: String,                         // UUID
    event_type: TelemetryEventType,     // Enum
    device_id: Option<String>,          // Which device
    timestamp: DateTime<Utc>,           // When
    data: serde_json::Value,            // Event payload
    agent_id: Option<String>,           // Who triggered
    severity: String,                   // info/warn/error
}

pub enum TelemetryEventType {
    DeviceDiscovered,
    Connected,
    Disconnected,
    AuthSuccess,
    AuthFailure,
    FrameCaptured,
    InputInjected,
    FileSynced,
    CapabilityGranted,
    CapabilityRevoked,
    Error,
    Metric,
}
```

**W&B Integration Example:**

```python
# Dashboard query
SELECT event_type, COUNT(*) as count
FROM telemetry_events
WHERE timestamp > now() - interval '1 hour'
GROUP BY event_type

# Alert on error rate
IF error_count / total_events > 0.05:
    ALERT('Error rate exceeded 5%')
```

**Universe Bridge:**

```rust
pub struct UniverseBridge {
    universe_tx: mpsc::UnboundedSender<UniverseEvent>,
}

emit_event(type, data)
emit_device_event(device_id, type, data)

// Creates causality chain for time-travel debugging
// Each event references parent for full trace
```

## Integration Architecture

### Tauri Command Layer

```rust
// In bonsai-workspace/src-tauri/src/android_commands.rs

#[tauri::command]
pub async fn android_list_devices(
    state: State<'_, AndroidBridge>,
) -> Result<Vec<DeviceInfo>, String> {
    state
        .get_device_pool()
        .get_all_devices()
        .iter()
        .map(|d| DeviceInfo {
            id: d.id.clone(),
            name: d.name.clone(),
            status: format!("{:?}", d.status),
            // ...
        })
        .collect()
}

#[tauri::command]
pub async fn android_connect(
    state: State<'_, AndroidBridge>,
    device_id: String,
) -> Result<String, String> {
    state.connect(&device_id).await.map(|h| h.device_id)
}

#[tauri::command]
pub async fn android_inject_touch(
    state: State<'_, AndroidBridge>,
    device_id: String,
    x: f32,
    y: f32,
) -> Result<(), String> {
    if let Some(device) = state.get_device_pool().get_device(&device_id) {
        // Inject touch event
    }
    Ok(())
}
```

### MCP Tool Layer

```json
{
  "tools": [
    {
      "name": "list_android_devices",
      "description": "List all discovered Android devices",
      "inputSchema": {
        "type": "object",
        "properties": {}
      }
    },
    {
      "name": "connect_android_device",
      "description": "Connect to an Android device",
      "inputSchema": {
        "type": "object",
        "properties": {
          "device_id": {"type": "string"}
        },
        "required": ["device_id"]
      }
    },
    {
      "name": "inject_input",
      "description": "Inject input event to device",
      "inputSchema": {
        "type": "object",
        "properties": {
          "device_id": {"type": "string"},
          "input_type": {"type": "string", "enum": ["touch", "keyboard", "pointer"]},
          "data": {"type": "object"}
        }
      }
    },
    {
      "name": "sync_files",
      "description": "Synchronize files between desktop and device",
      "inputSchema": {
        "type": "object",
        "properties": {
          "device_id": {"type": "string"},
          "direction": {"type": "string", "enum": ["push", "pull", "bidirectional"]}
        }
      }
    },
    {
      "name": "grant_capability",
      "description": "Issue a capability token for a device",
      "inputSchema": {
        "type": "object",
        "properties": {
          "device_id": {"type": "string"},
          "subject": {"type": "string"},
          "capability": {"type": "string"},
          "duration_hours": {"type": "integer"}
        }
      }
    }
  ]
}
```

## Data Flow Diagrams

### Connection Establishment

```
User (IDE)
    │
    ├─ Click "Connect Device"
    │
    ▼
Tauri Command Layer
    │
    ├─ validate inputs
    │
    ▼
AndroidBridge::connect(device_id)
    │
    ├─ Find in registry
    ├─ Create Device instance
    ├─ Add to DevicePool
    │
    ▼
TransferDaemon::establish_session()
    │
    ├─ DNS lookup (device IP)
    ├─ TCP connect to device:port
    │
    ▼
Noise Protocol Handshake
    │
    ├─ Message 1: Device sends ephemeral + encrypted identity
    ├─ Message 2: Bridge responds with ephemeral + signature
    ├─ Message 3: Bridge sends static identity + capabilities
    │
    ▼
Session Established
    │
    ├─ AES-256-GCM symmetrical keys ready
    ├─ Device responds with capabilities
    ├─ Capability tokens issued
    │
    ▼
Connected State
    │
    ├─ Emit TelemetryEvent::Connected
    ├─ Emit UniverseEvent::DeviceConnected
    │
    ▼
Return ConnectionHandle to user
```

### Screen Streaming

```
Android Device
    │
    ├─ SurfaceFlinger captures frame
    ├─ Encodes with H.265 (hardware)
    │
    ▼
ScreenFrame { data, sequence, codec }
    │
    ├─ Serialize to JSON
    ├─ Encrypt with SessionKey (AES-256-GCM)
    │
    ▼
TransferDaemon
    │
    ├─ Send over TCP
    │
    ▼
Bridge::ScreenStreamer
    │
    ├─ Decrypt frame
    ├─ Validate checksum
    ├─ Enqueue to frame_queue
    ├─ Record metrics (latency, fps)
    │
    ▼
Tauri Event Emitter
    │
    ├─ Emit frame to UI
    │
    ▼
Svelte Component
    │
    ├─ Decode H.265 (WASM decoder)
    ├─ Render to Canvas
    │
    ▼
User sees live screen
```

### Capability-Based Access Control

```
Agent requests: "Inject touch at (500, 1000)"
    │
    ├─ Submit with token_id
    │
    ▼
Bridge::inject_input(device_id, token_id, event)
    │
    ├─ Look up CapabilityToken
    ├─ Verify token.is_valid() (not expired, not revoked)
    ├─ Verify token.device_id == device_id
    ├─ Verify token.capability == InputInjection
    ├─ Verify token.signature (Ed25519)
    │
    ├─ If all checks pass:
    │  ├─ Create InputEvent
    │  ├─ Serialize and encrypt
    │  ├─ Send to device
    │  ├─ Emit InputInjected telemetry event
    │  └─ Return success
    │
    └─ If any check fails:
       ├─ Emit CapabilityCheckFailed event
       ├─ Log with severity=WARN
       └─ Return error
```

## Security & Threat Model

### Threat: Unauthorized Device Access

**Mitigation:**
- Capability tokens required for every operation
- Tokens time-bounded (default 24 hours)
- Signatures verified (Ed25519)
- Can be revoked instantly

**Detection:**
- Track failed capability checks
- Alert on high failure rate
- Log all capability denials

### Threat: Man-in-the-Middle

**Mitigation:**
- Noise protocol with forward secrecy
- Public key pinning (device fingerprint)
- AES-256-GCM with authenticated encryption
- Nonce counter prevents replay

**Detection:**
- Unexpected public keys trigger alert
- Connection failures logged
- Hash mismatches detected

### Threat: Privilege Escalation

**Mitigation:**
- Capabilities are granular (not all-or-nothing)
- Each capability is signed
- Cannot escalate without new token
- Old tokens remain revocable

**Detection:**
- Track token issuance history
- Alert on unusual capability requests
- Monitor for token abuse patterns

### Threat: Denial of Service

**Mitigation:**
- Rate limiting per device
- Input validation on all messages
- Resource quotas (buffer sizes)
- Heartbeat timeouts

**Detection:**
- Monitor frame drop rate
- Alert on connection instability
- Track error rates by type

## Performance Characteristics

### Memory Usage (Per Device)

```
Device struct:              ~200 bytes
Identity (keypairs):        ~100 bytes
Capabilities (average 5):   ~500 bytes
Metrics:                    ~200 bytes
Buffers:                    ~4 MB (frame queue)
──────────────────────────────────
Total per device:           ~5 MB
```

**For 1000 devices:** ~5 GB RAM (plus overhead)

### Network Usage (Per Device)

```
Heartbeat (10s interval):       100 bytes
Screen frame (H.265, 60 fps):   ~50 KB/frame
Input event (avg):               100 bytes
File sync (sparse):              Variable
──────────────────────────────────
Typical usage:                  ~30 Mbps @ 60fps

Adaptive range:                 1-20 Mbps
```

### CPU Usage (Per Device)

```
Decryption (per frame):         ~1% (hardware accelerated)
Metric updates:                 <1%
Event serialization:            <1%
Background tasks:               ~2% (heartbeat, cleanup)
──────────────────────────────────
Total per device:               ~5% per core
```

## Deployment Patterns

### Single Bridge (Development)

```
Desktop Machine
├─ IDE (Tauri)
├─ AndroidBridge (single instance)
└─ Up to 10 connected devices
```

### Multi-Bridge (Production)

```
Control Plane
├─ Coordinator service
├─ Shared capability registry (database)
├─ Shared telemetry aggregation
└─ Load balancer

Data Plane (N instances)
├─ Bridge #1 (devices 1-100)
├─ Bridge #2 (devices 101-200)
└─ Bridge #N (devices N*100+1..(N+1)*100)

Load distribution: Round-robin or device hash
Failover: Automatic bridge restart
```

### Kubernetes Deployment

```yaml
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: bonsai-android-bridge
spec:
  replicas: 10  # 10 bridges, 100 devices each
  template:
    spec:
      containers:
      - name: bridge
        image: bonsai:android-bridge
        resources:
          requests:
            memory: 2Gi
            cpu: 2
          limits:
            memory: 4Gi
            cpu: 4
        ports:
        - containerPort: 5037  # Device connections
        env:
        - name: DEVICES_PER_BRIDGE
          value: "100"
        - name: CAPABILITY_DB
          value: "postgresql://postgres:5432/capabilities"
```

## Future Enhancements

### Phase 2: Advanced Streaming
- [ ] WebRTC for P2P streaming (bypass bridge)
- [ ] GPU-accelerated encoding (NVENC, Apple Video Toolbox)
- [ ] Perceptual quality metrics (VMAF, SSIM)
- [ ] Multi-bitrate profiles (temporal/spatial scalability)

### Phase 3: Advanced App Management
- [ ] APK delta patching (90% size reduction)
- [ ] App hot-reload without restart
- [ ] A/B testing infrastructure
- [ ] Crash reporting integration

### Phase 4: Sensor & Hardware
- [ ] Stream sensor data (GPS, IMU, accelerometer)
- [ ] Battery/temperature monitoring
- [ ] Hardware component testing
- [ ] Peripheral control (camera, microphone)

### Phase 5: Intelligence
- [ ] ML-based anomaly detection
- [ ] Automatic issue diagnosis
- [ ] Predictive device failure detection
- [ ] Cost optimization recommendations

## Conclusion

The Android Bridge provides a production-grade foundation for mobile device management at scale. Its zero-trust security model, high performance, and comprehensive observability make it suitable for enterprise deployments, research environments, and developer workflows.
