# Bonsai Android Bridge

Production-grade Android Bridge for the Bonsai Ecosystem. A comprehensive system for discovering, connecting to, and controlling Android devices with enterprise-level security, performance, and observability.

## Key Features

### Core Capabilities
- **Zero-Trust Security**: Capability-based access control with signed tokens
- **High-Performance Screen Streaming**: H.264/H.265 with adaptive bitrate (<50ms latency target)
- **Input Injection**: Touch, keyboard, and mouse input synthesis
- **File Synchronization**: Content-addressed delta sync using BLAKE3
- **Device Management**: Multi-device coordination with 1-1000+ scalability
- **App Deployment**: APK installation and hot-reload support
- **Sensor Access**: GPS, accelerometer, gyroscope integration

### Integration
- **TransferDaemon**: End-to-end encryption with Noise protocol
- **Bonsai Universe**: Time-travel debugging and event causality
- **UACS**: Human-in-the-loop approval for sensitive operations
- **W&B Integration**: Real-time telemetry and metrics
- **MCP Tools**: Agentic interface for orchestration

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Bonsai IDE (Tauri)                        │
│         [Android Panel Component] [Telemetry Dashboard]      │
└──────────────────────┬───────────────────────────────────────┘
                       │ Tauri Commands
┌──────────────────────▼───────────────────────────────────────┐
│                   AndroidBridge                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Discovery (mDNS + Manual Registry)                      │ │
│  │ Capability Registry (Zero-Trust)                        │ │
│  │ Device Pool (1000+ concurrent)                          │ │
│  │ Telemetry Collector (W&B + Universe)                    │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────┬──────────┬──────────┬──────────┬──────────────────────┘
      │          │          │          │
   Screen    Input      File Sync   Capability
   Stream    Inject                Management
      │          │          │          │
┌─────▼──────────▼──────────▼──────────▼──────────────────────┐
│              TransferDaemon (Noise Protocol)                │
│    End-to-End Encrypted Channel + Session Management       │
└─────┬──────────────────────────────────────────────────────┘
      │ TCP/WebSocket
┌─────▼──────────────────────────────────────────────────────┐
│          Android Device (via ADB or native bridge)          │
│  ┌─────────────────────────────────────────────────────────┤
│  │ BonsaiAgent (Kotlin)                                    │ │
│  │ ├─ Screen Encoder (H.264/H.265)                        │ │
│  │ ├─ Input Handler (Touch/Keyboard/Mouse)                │ │
│  │ ├─ File Sync Service                                   │ │
│  │ ├─ App Deployer                                        │ │
│  │ └─ Sensor Provider                                     │ │
│  └─────────────────────────────────────────────────────────┤ │
└────────────────────────────────────────────────────────────┘
```

## Module Organization

### `connection.rs`
Main orchestrator coordinating all subsystems.

```rust
pub struct AndroidBridge {
    device_pool: Arc<DevicePool>,
    discovery: Arc<DiscoveryService>,
    registry: Arc<ManualDeviceRegistry>,
    capability_registry: Arc<CapabilityRegistry>,
    telemetry: Arc<TelemetryCollector>,
    identity: DeviceIdentity,
}
```

**Key Methods:**
- `initialize()` - Start all services
- `register_device()` - Add device (mDNS or manual)
- `connect()` - Establish secure connection
- `disconnect()` - Clean shutdown
- `issue_capability()` - Grant time-bound capabilities
- `revoke_capability()` - Revoke access

### `discovery.rs`
Device discovery with mDNS support and fallback manual registry.

```rust
pub struct DiscoveryService {
    devices: Arc<RwLock<Vec<DiscoveredDevice>>>,
    discovery_interval: Duration,
}

pub struct ManualDeviceRegistry {
    devices: Arc<RwLock<HashMap<String, DiscoveredDevice>>>,
}
```

### `capability.rs`
Zero-trust capability-based access control.

```rust
pub struct CapabilityToken {
    id: Uuid,
    capability: CapabilityType,
    device_id: String,
    subject: String,
    issued_at: DateTime,
    expires_at: DateTime,
    signature: Vec<u8>,  // Ed25519
}

pub enum CapabilityType {
    ScreenStream,
    InputInjection,
    FileRead,
    FileWrite,
    AppDeploy,
    SensorAccess,
    // ...
}
```

### `security.rs`
Noise protocol implementation and cryptographic primitives.

```rust
pub struct DeviceIdentity {
    fingerprint: String,
    secret_key: StaticSecret,  // X25519
    public_key: PublicKey,
}

pub struct SessionKey {
    key: [u8; 32],
    nonce_counter: Arc<Mutex<u64>>,
}
```

**Encryption:** AES-256-GCM with automatic nonce management

### `device.rs`
Device state management and metrics.

```rust
pub struct Device {
    id: String,
    status: DeviceStatus,
    ip: String,
    port: u16,
    identity: Arc<DeviceIdentity>,
    capabilities: Vec<CapabilityType>,
    metrics: DeviceMetrics,
}

pub struct DevicePool {
    devices: Arc<RwLock<HashMap<String, Device>>>,
}
```

### `streaming.rs`
Screen streaming with adaptive bitrate.

```rust
pub struct ScreenStreamer {
    config: Arc<RwLock<BitrateConfig>>,
    network_metrics: Arc<RwLock<NetworkMetrics>>,
    codec: VideoCodec,
}

pub struct ScreenFrame {
    timestamp_us: u64,
    sequence: u64,
    frame_data: Vec<u8>,
    is_keyframe: bool,
    checksum: u32,
}
```

**Features:**
- H.264 & H.265 support
- Adaptive bitrate (1-20 Mbps)
- Frame checksums for integrity
- <50ms latency target

### `input.rs`
Multi-modal input injection.

```rust
pub struct InputInjector {
    event_queue: mpsc::UnboundedSender<InputEvent>,
    event_counter: Arc<Mutex<u32>>,
}
```

**Supports:**
- Touch events (multi-pointer)
- Keyboard input (with modifiers)
- Mouse/pointer events
- Text injection
- Gesture synthesis (swipe, tap, etc.)

### `file_sync.rs`
Content-addressed file synchronization.

```rust
pub struct FileSynchronizer {
    sync_root: PathBuf,
    metadata_cache: Arc<RwLock<HashMap<String, FileMetadata>>>,
    pending_ops: mpsc::UnboundedSender<FileSyncOp>,
}

pub struct FileSyncOp {
    event_type: FileSyncEventType,
    metadata: FileMetadata,
    blob_ref: Option<BlobRef>,  // BLAKE3 hash
    delta_blocks: Option<Vec<DeltaBlock>>,
}
```

**Features:**
- Incremental delta sync
- BLAKE3 content hashing
- Bi-directional synchronization
- Compression support

### `telemetry.rs`
Observable event logging with W&B and Universe integration.

```rust
pub struct TelemetryCollector {
    events: Arc<RwLock<Vec<TelemetryEvent>>>,
    event_tx: mpsc::UnboundedSender<TelemetryEvent>,
}

pub struct TelemetryEvent {
    id: String,
    event_type: TelemetryEventType,
    device_id: Option<String>,
    data: Value,
    severity: String,
}
```

## Security Model

### Zero-Trust Principles
1. **No implicit trust** - All operations require explicit capability tokens
2. **Signed tokens** - Ed25519 signatures on all capabilities
3. **Time-bounded** - Tokens expire automatically
4. **Scope-aware** - Capabilities can be scoped to specific resources
5. **Revocable** - Instant revocation with no propagation delay

### Encryption
- **Transport**: Noise protocol (IK pattern) for session establishment
- **Session**: AES-256-GCM with automatic nonce management
- **Authentication**: X25519 ECDH with Ed25519 signing

### Defense in Depth
- Capability checks on every operation
- Input validation on all messages
- Rate limiting per device
- Anomaly detection via telemetry
- UACS integration for sensitive operations

## Integration Points

### Tauri Commands
```rust
// In bonsai-workspace/src-tauri/src/android_commands.rs
#[tauri::command]
pub async fn get_devices(state: State<AndroidBridge>) -> Result<Vec<Device>> {
    Ok(state.get_device_pool().get_all_devices())
}

#[tauri::command]
pub async fn connect_device(
    state: State<AndroidBridge>,
    device_id: String,
) -> Result<String> {
    let handle = state.connect(&device_id).await?;
    Ok(handle.device_id)
}
```

### MCP Tools
```python
# In Anthropic Claude API
tools = [
    {
        "name": "list_android_devices",
        "description": "List connected Android devices",
    },
    {
        "name": "inject_input",
        "description": "Inject touch/keyboard input to device",
        "input_schema": {
            "type": "object",
            "properties": {
                "device_id": {"type": "string"},
                "input_type": {"type": "string"},
                "data": {"type": "object"},
            }
        }
    },
    # ... more tools
]
```

### Bonsai Universe Integration
```rust
universe_bridge.emit_device_event(
    &device_id,
    "connection_established",
    json!({
        "ip": "192.168.1.100",
        "api_level": 31,
        "capabilities": ["ScreenStream", "InputInjection"],
    })
);
```

## Usage Examples

### Basic Connection
```rust
use bonsai_android_bridge::AndroidBridge;

let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
let telemetry = TelemetryCollector::new(tx, 1000);
let bridge = AndroidBridge::new(telemetry, Duration::from_secs(5));

bridge.initialize().await?;
bridge.register_device(
    "device1".to_string(),
    "Pixel 6".to_string(),
    "Pixel 6".to_string(),
    31,
    "192.168.1.100".to_string(),
    5037,
    "pk_xyz".to_string(),
)?;

let mut handle = bridge.connect("device1").await?;
```

### Screen Streaming
```rust
let streamer = handle.create_screen_streamer(BitrateConfig::default())?;
streamer.start().await?;

// Receive frames in event loop
while let Some(frame) = receiver.recv().await {
    println!("Frame {}: {}x{}", frame.sequence, frame.width, frame.height);
}
```

### Input Injection
```rust
let injector = handle.create_input_injector()?;

// Single tap
injector.click(500.0, 1000.0).await?;

// Multi-step gesture
injector.swipe(100.0, 500.0, 500.0, 500.0, 500).await?;

// Text input
injector.inject_text("Hello Android").await?;
```

### Capability Management
```rust
let token_id = bridge
    .issue_capability("device1", "agent1", CapabilityType::ScreenStream, 24)
    .await?;

let has_access = bridge.check_capability(
    "device1",
    "agent1",
    &CapabilityType::ScreenStream,
);

bridge.revoke_capability(&token_id).await?;
```

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Screen Latency | <50ms | End-to-end from frame capture to display |
| Input Latency | <30ms | Touch to on-device processing |
| Discovery Time | <2s | mDNS broadcast to device listed |
| Connection Setup | <500ms | Noise handshake + capability negotiation |
| File Sync | >10 MB/s | Over LAN with gigabit Ethernet |
| Max Devices | 1000+ | Per bridge instance |
| Memory Per Device | ~5 MB | Baseline state + buffers |

## Scalability

### Device Scaling
```rust
// Multi-bridge architecture for >100 devices
let bridges = vec![
    AndroidBridge::new(telemetry.clone(), Duration::from_secs(5)),
    AndroidBridge::new(telemetry.clone(), Duration::from_secs(5)),
    // ... distribute devices across bridges
];
```

### Load Distribution
- Round-robin device assignment
- Per-bridge telemetry aggregation
- Federated capability registry
- Shared CAS blob store

## Observability

### Metrics (W&B)
- Frame capture rate and latency
- Input event throughput
- File sync progress
- Device connection stability
- Capability token usage
- Error rates by type

### Events (Universe)
- Device discovered/connected/disconnected
- Capability granted/revoked/expired
- Input injected (with coordinates)
- Files synced (with hashes)
- Errors with full context

### Logging
```
[DEBUG] Device #234: heartbeat OK (rtt=12ms)
[INFO] Frame #1234 captured: 1080x2400@60fps, latency=45ms
[WARN] Bandwidth degraded: 2.5 Mbps -> 1.8 Mbps
[ERROR] Capability check failed: device1 lacks FileWrite
```

## Android Agent (Kotlin)

The companion agent running on Android devices:

```kotlin
// Minimal Kotlin agent (full implementation in android-agent/)
class BonsaiAgent(val context: Context) {
    private val screenEncoder = ScreenEncoder()
    private val inputHandler = InputHandler()
    private val fileSyncService = FileSyncService()
    
    fun startBridge(bridgeAddress: String, port: Int) {
        val connection = NoiseConnection(bridgeAddress, port)
        screenEncoder.start(connection)
        inputHandler.listen(connection)
        fileSyncService.sync(connection)
    }
}
```

## Testing

```bash
# Unit tests
cargo test --lib

# Integration tests (requires Android device/emulator)
cargo test --test '*' -- --ignored --nocapture

# Benchmarks
cargo bench

# Security audit
cargo audit

# Coverage
cargo tarpaulin --out Html
```

## Future Enhancements

- [ ] WebRTC for direct P2P streaming
- [ ] App hot-reload with delta patching
- [ ] Sensor data streaming (GPS, IMU)
- [ ] GPU-accelerated video encoding
- [ ] Machine learning-based anomaly detection
- [ ] Federated multi-region deployment
- [ ] Kubernetes operator for orchestration

## Dependencies

### Core
- `tokio` - Async runtime
- `serde/serde_json` - Serialization
- `blake3` - Content hashing
- `ed25519-dalek` - Digital signatures
- `x25519-dalek` - Key agreement
- `aes-gcm` - Encryption

### Integration
- `p2p-crypto` - Noise protocol
- `bonsai-capability-registry` - Capability model
- `audit-log` - Event logging
- `bonsai-cas` - Content addressing

## License

Bonsai Ecosystem

## Contributors

- Claude Code (Initial implementation)
- Bonsai Engineering Team

## References

- [Noise Protocol](https://noiseprotocol.org/)
- [Capability-Based Security](https://en.wikipedia.org/wiki/Capability-based_security)
- [BLAKE3](https://blake3.io/)
- [Android Developer Guide](https://developer.android.com/)
