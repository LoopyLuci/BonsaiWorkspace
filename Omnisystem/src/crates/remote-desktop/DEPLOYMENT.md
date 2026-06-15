# BRDF Production Deployment Guide

## System Requirements

### Minimum

- **CPU**: 2 cores, 2 GHz
- **Memory**: 512 MB
- **Storage**: 100 MB
- **Network**: Stable 1 Mbps (0.5 Mbps minimum)
- **OS**: Linux, macOS, Windows 10+

### Recommended

- **CPU**: 4+ cores, 2.4+ GHz
- **Memory**: 2+ GB
- **Storage**: 1 GB SSD
- **Network**: 25 Mbps+ with <50ms latency
- **OS**: Ubuntu 20.04+, macOS 10.14+, Windows Server 2019+

### Hardware Acceleration

For optimal performance, enable hardware encoding:

**Windows**:
- NVIDIA: CUDA 11.0+, GTX 750 or better
- AMD: Radeon RX 460 or better
- Intel: 6th gen or newer with Quick Sync

**macOS**:
- T2 chip or Apple Silicon (built-in)
- Older Macs: requires external GPU

**Linux**:
- NVIDIA: CUDA 11.0+
- AMD: VAAPI support (kernel 5.4+)
- Intel: VA-API or Quick Sync

## Installation

### From Source

```bash
# Clone repository
git clone https://github.com/bonsai-ai/bonsai
cd bonsai

# Build release
cargo build -p bonsai-remote-desktop --release

# Binary location
./target/release/bonsai-remote-desktop
```

### Environment Variables

```bash
# Enable debug logging
export RUST_LOG=debug,bonsai_remote_desktop=trace

# Hardware acceleration (Windows)
export NVENC_PATH=/path/to/nvidia-codec-sdk

# Network configuration
export BRDF_STUN_SERVER=stun.l.google.com:19302
export BRDF_RELAY_ADDR=0.0.0.0:3389
export BRDF_MDNS_PORT=5353

# TLS certificates
export BRDF_CERT_PATH=/etc/bonsai-rd/certs
export BRDF_KEY_PATH=/etc/bonsai-rd/keys
```

## Configuration

### config.toml

```toml
[network]
# Rendezvous (discovery) settings
rendezvous_port = 5353          # mDNS port
stun_servers = [
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
]
nat_timeout_secs = 5

# Relay settings
relay_listen_addr = "0.0.0.0:3389"
relay_max_connections = 1000
relay_timeout_secs = 300

[media]
# Capture settings
capture_fps = 60
capture_resolution = "1920x1080"
capture_quality = "high"        # low, medium, high

# Encoding settings
default_codec = "h265"          # h264, h265, vp8, vp9, av1
hardware_acceleration = true
max_bitrate_mbps = 50.0
min_bitrate_mbps = 0.5

[security]
# Token settings
token_lifetime_hours = 24
require_https = true
tls_cert_path = "/etc/bonsai-rd/certs/server.pem"
tls_key_path = "/etc/bonsai-rd/keys/server.key"

# Session settings
max_sessions_per_peer = 10
session_idle_timeout_secs = 1800
require_capability_token = true

[telemetry]
# Universe integration
universe_db_path = "/var/lib/bonsai-rd/universe.db"
event_retention_days = 30
enable_event_logging = true

# Monitoring
metrics_port = 9090
prometheus_enabled = true

[performance]
# Relay settings
relay_buffer_size_mb = 10
relay_thread_pool_size = 4

# Capture settings
capture_queue_size = 60         # Frames to buffer

# Encoding settings
encoding_queue_size = 30        # Encoded frames to buffer
```

## Network Configuration

### Firewall Rules

```bash
# Allow mDNS discovery (UDP port 5353)
sudo ufw allow 5353/udp

# Allow RDP relay (TCP port 3389)
sudo ufw allow 3389/tcp

# Allow HTTPS management (TCP port 443)
sudo ufw allow 443/tcp

# Restrict to trusted networks
sudo ufw allow from 192.168.1.0/24 to any port 3389 proto tcp
```

### Port Mapping (NAT)

For devices behind NAT:

```bash
# UPnP (automatic)
# Enable in config: upnp_enabled = true

# Manual port forwarding
# Router → Port 3389 (external) → Device IP:3389 (internal)

# Verify connectivity
netstat -tlnp | grep 3389
```

### DNS Configuration

```bash
# For relay discovery, configure DNS:
_bonsai-rd._tcp.local.    SRV    0 0 3389 your-device.local.

# Or use fixed IP
relay.bonsai-rd.internal  A      192.168.1.100
```

## Security Hardening

### TLS/SSL Setup

```bash
# Generate self-signed certificate (development)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365

# Or use Let's Encrypt (production)
sudo certbot certonly --standalone -d relay.yourdomain.com
```

### Ed25519 Key Generation

```bash
# Generate signing key
openssl genpkey -algorithm ed25519 -out private-key.pem

# Extract public key
openssl pkey -in private-key.pem -pubout -out public-key.pem
```

### Capability Token Management

```bash
# Create token (script)
#!/bin/bash
PEER_ID="peer-abc123def456"
CAPABILITIES="connect,capture,input"
DURATION="24h"

# Use bonsai-cli or SDK to create token
bonsai-rd token create \
    --peer "$PEER_ID" \
    --capabilities "$CAPABILITIES" \
    --duration "$DURATION" \
    --sign-key "private-key.pem"
```

### SELinux Policy (Optional)

```bash
# Create policy for BRDF
sudo semanage fcontext -a -t bin_t "/usr/local/bin/bonsai-rd"
sudo semanage port -a -t http_port_t -p tcp 3389
sudo semanage port -a -t http_port_t -p udp 5353
restorecon -Rv /usr/local/bin/bonsai-rd
```

## Monitoring & Observability

### Prometheus Metrics

```bash
# Enable Prometheus scraping
curl http://localhost:9090/metrics

# Key metrics:
# - bonsai_rd_peers_total          # Registered peers
# - bonsai_rd_sessions_active      # Active sessions
# - bonsai_rd_bitrate_mbps         # Current bitrate
# - bonsai_rd_packet_loss_percent  # Network quality
# - bonsai_rd_relay_latency_ms     # Relay latency
```

### Log Monitoring

```bash
# Enable structured logging
export RUST_LOG=bonsai_remote_desktop=debug

# View logs
journalctl -u bonsai-remote-desktop -f

# Log file rotation
logrotate -f /etc/logrotate.d/bonsai-remote-desktop

# Key log events to monitor:
# - Token verification failures
# - NAT hole punching failures
# - Relay connection errors
# - Session timeouts
```

### Universe Event Monitoring

```bash
# Query Universe store
bonsai-cli universe query \
    --filter "source=RemoteDesktop" \
    --limit 100

# Monitor security events
bonsai-cli universe query \
    --filter "category=SecurityEvent" \
    --since "1 hour ago"

# Session analytics
bonsai-cli universe analytics \
    --metric "session_duration" \
    --group-by "peer_id"
```

## Troubleshooting

### Connection Issues

```bash
# Test peer discovery
ping _bonsai-rd._tcp.local

# Check relay connectivity
telnet relay.yourdomain.com 3389

# Verify NAT hole punching
stun-client stun.l.google.com 19302

# Check firewall rules
sudo iptables -L | grep 3389
```

### Performance Issues

```bash
# Check CPU usage
top -p $(pgrep bonsai-rd)

# Monitor memory
ps aux | grep bonsai-rd | grep -v grep

# Check network statistics
ss -tlnp | grep bonsai-rd

# Enable profiling
export BRDF_PROFILE=1
```

### Codec Issues

```bash
# List available codecs
bonsai-rd codecs --list

# Test encoding
bonsai-rd encode --test --codec h265 --bitrate 5.0

# Check hardware acceleration
bonsai-rd encode --test --use-hwaccel

# Fall back to software encoding
export BRDF_DISABLE_HWACCEL=1
```

## Scaling & Load Balancing

### Single Server

- Max ~100 concurrent sessions
- Max ~10,000 Mbps total bandwidth
- Monitor relay thread pool (default 4)

### Multiple Relay Servers

```bash
# Configure mesh of relays
[relay_peers]
peer1 = "relay1.yourdomain.com:3389"
peer2 = "relay2.yourdomain.com:3389"
peer3 = "relay3.yourdomain.com:3389"

# Load balancing (DNS round-robin)
relay.yourdomain.com  A  10.0.0.1
relay.yourdomain.com  A  10.0.0.2
relay.yourdomain.com  A  10.0.0.3
```

### Session Distribution

```toml
[clustering]
mode = "mesh"               # peer-to-peer mesh
heartbeat_interval_secs = 5
session_rebalance = true
rebalance_threshold = 0.8   # Rebalance if >80% loaded
```

## Backup & Recovery

### Backing Up Universe Events

```bash
# Export events
bonsai-cli universe export \
    --output "/backup/universe-$(date +%Y%m%d).json" \
    --format json

# Schedule daily backup
0 2 * * * bonsai-cli universe export \
    --output "/backup/universe-$(date +\%Y\%m\%d).json"
```

### Session Recovery

```bash
# Force-close stale sessions
bonsai-rd admin gc --force

# Recover from relay crash
bonsai-rd admin repair --recover-sessions

# Verify integrity
bonsai-rd admin verify
```

## Compliance

### HIPAA Compliance

```bash
# Enable FIPS mode
export OPENSSL_FIPS=1

# Use FIPS-approved ciphers only
[security.tls]
cipher_suites = ["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384"]

# Audit logging
[telemetry]
audit_log_path = "/var/log/bonsai-rd/audit.log"
audit_log_retention_days = 365
```

### SOC 2 Requirements

- ✅ Encryption in transit (TLS 1.3)
- ✅ Encryption at rest (AES-GCM)
- ✅ Cryptographic key management
- ✅ Comprehensive audit logging
- ✅ Access control (capability tokens)
- ✅ Incident response procedures

## Performance Tuning

### For High Latency Networks

```toml
[performance]
# Increase buffers
relay_buffer_size_mb = 50
capture_queue_size = 120

# Adjust timeouts
relay_timeout_secs = 600
nat_timeout_secs = 15

# Use low-latency codecs
default_codec = "h264"
```

### For High Throughput

```toml
[performance]
relay_thread_pool_size = 8      # More threads
capture_queue_size = 30         # Smaller queue (discard frames)
encoding_queue_size = 10        # Prioritize latency

[media]
max_bitrate_mbps = 100
capture_fps = 120               # Higher FPS
```

### Memory-Constrained Devices

```toml
[performance]
relay_buffer_size_mb = 2
capture_queue_size = 10
encoding_queue_size = 5

[media]
capture_resolution = "1280x720"
capture_fps = 30
default_codec = "vp9"          # Better compression
max_bitrate_mbps = 2
```

## Update Procedure

```bash
# 1. Backup configuration and universe events
cp /etc/bonsai-rd/config.toml /backup/config-$(date +%Y%m%d).toml
bonsai-cli universe export --output /backup/universe-$(date +%Y%m%d).json

# 2. Stop service
sudo systemctl stop bonsai-remote-desktop

# 3. Build new version
cargo build -p bonsai-remote-desktop --release

# 4. Backup old binary
cp /usr/local/bin/bonsai-rd /usr/local/bin/bonsai-rd.old

# 5. Install new binary
sudo cp target/release/bonsai-remote-desktop /usr/local/bin/bonsai-rd

# 6. Run migrations (if any)
bonsai-rd migrate --from 0.1.0

# 7. Start service
sudo systemctl start bonsai-remote-desktop

# 8. Verify
sleep 5 && curl http://localhost:9090/metrics | head -20
```

## Systemd Service File

```ini
[Unit]
Description=Bonsai Remote Desktop Fabric
After=network.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/bonsai-rd --config /etc/bonsai-rd/config.toml
Restart=on-failure
RestartSec=5
User=bonsai-rd
Group=bonsai-rd

# Security hardening
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/bonsai-rd /var/log/bonsai-rd

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=bonsai-rd

[Install]
WantedBy=multi-user.target
```

## Status: Production Ready

✅ All components tested and verified for production use
