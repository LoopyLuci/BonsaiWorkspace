# Android Bridge Deployment & Operations Guide

## Production Deployment

### Overview

The Android Bridge is designed for enterprise-scale deployment supporting 1-1000+ devices. This guide covers architecture decisions, deployment patterns, monitoring, and operational procedures.

## Architecture Patterns

### Single Machine (Development)

**Setup:** IDE host runs one bridge instance

```
┌─────────────────────────────────┐
│ Developer Machine               │
├─────────────────────────────────┤
│ Tauri IDE (Svelte UI)           │
│ AndroidBridge (1 instance)      │
│ Up to 10 concurrent devices     │
│ ├─ Pixel 6 (192.168.1.10)      │
│ ├─ Galaxy S21 (192.168.1.11)   │
│ └─ Nexus 5X (192.168.1.12)     │
└─────────────────────────────────┘
```

**Deployment:**
```bash
# Build workspace with bridge
cargo build --release --workspace

# Run Tauri app
cd bonsai-workspace
npm run tauri dev

# Bridge auto-initializes on startup
```

### Multi-Bridge (Production, 100-1000+ Devices)

**Architecture:**

```
┌──────────────────────────────────────────────────────────────┐
│ Control Plane (Shared Services)                              │
├──────────────────────────────────────────────────────────────┤
│ ┌────────────────────┐  ┌─────────────────┐  ┌────────────┐ │
│ │ Load Balancer      │  │ Capability DB   │  │ Metrics    │ │
│ │ (Round-robin)      │  │ (PostgreSQL)    │  │ Store      │ │
│ └────────────────────┘  └─────────────────┘  │ (W&B/TSD)  │ │
│                                                └────────────┘ │
└─────────┬──────────────────────────────────────────────────────┘
          │ TCP 5037 (device connections)
          │ TCP 8080 (API)
          │
  ┌───────┴───────┬──────────────┬──────────────┐
  │               │              │              │
  ▼               ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│ Bridge #1│  │ Bridge #2│  │ Bridge #N│  │ Bridge M │
│ Inst-1   │  │ Inst-1   │  │ Inst-1   │  │ Inst-1   │
│  Dev:1-  │  │  Dev:101 │  │  Dev:201 │  │ Dev:301+ │
│  100     │  │  -200    │  │  -300    │  │          │
└─────┬────┘  └─────┬────┘  └─────┬────┘  └─────┬────┘
      │             │             │             │
      └─────────────┴─────────────┴─────────────┘
              (Local Network)
              1000+ Android Devices
```

### Kubernetes Deployment

**Manifest Structure:**

```yaml
# kubernetes/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: bonsai-android

---
# kubernetes/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: bridge-config
  namespace: bonsai-android
data:
  devices_per_instance: "100"
  screen_bitrate_kbps: "5000"
  heartbeat_interval_secs: "10"
  capability_ttl_hours: "24"

---
# kubernetes/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: android-bridge
  namespace: bonsai-android
spec:
  replicas: 10  # Scale to 10 instances for 1000 devices
  selector:
    matchLabels:
      app: android-bridge
  template:
    metadata:
      labels:
        app: android-bridge
    spec:
      containers:
      - name: bridge
        image: bonsai:android-bridge:latest
        imagePullPolicy: Always
        ports:
        - containerPort: 5037
          name: device-port
        - containerPort: 8080
          name: api-port
        env:
        - name: INSTANCE_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: CAPABILITY_DB
          valueFrom:
            secretKeyRef:
              name: db-credentials
              key: connection-string
        - name: TELEMETRY_ENDPOINT
          value: "https://api.wandb.ai/graphql"
        resources:
          requests:
            memory: "2Gi"
            cpu: "2"
          limits:
            memory: "4Gi"
            cpu: "4"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - android-bridge
              topologyKey: kubernetes.io/hostname

---
# kubernetes/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: android-bridge-lb
  namespace: bonsai-android
spec:
  type: LoadBalancer
  selector:
    app: android-bridge
  ports:
  - port: 5037
    targetPort: 5037
    protocol: TCP
    name: device-port
  - port: 8080
    targetPort: 8080
    protocol: TCP
    name: api-port

---
# kubernetes/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: bridge-hpa
  namespace: bonsai-android
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: android-bridge
  minReplicas: 5
  maxReplicas: 50
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
```

## Deployment Procedures

### Pre-Deployment Checklist

- [ ] All tests passing: `cargo test --workspace`
- [ ] Android Agent APK built and signed
- [ ] Certificates for Noise protocol prepared
- [ ] Database migrations applied
- [ ] Capability registry initialized
- [ ] Telemetry endpoints verified
- [ ] Security policies reviewed

### Step 1: Build Bridge Artifacts

```bash
# Build Rust bridge binary
cd crates/bonsai-android-bridge
cargo build --release

# Output: target/release/libbonsai_android_bridge.*

# Build Android Agent APK
cd ../../android-agent
./gradlew bundleRelease

# Output: app/build/outputs/bundle/release/app.aab
```

### Step 2: Container Build (Kubernetes)

```bash
# Create Dockerfile
cat > Dockerfile <<'EOF'
FROM rust:1.70 as builder
WORKDIR /build
COPY . .
RUN cargo build --release --package bonsai-android-bridge

FROM ubuntu:22.04
RUN apt-get update && apt-get install -y libssl3
COPY --from=builder /build/target/release/libbonsai_android_bridge.so /app/
COPY --from=builder /build/crates/bonsai-android-bridge/config/ /app/config/
EXPOSE 5037 8080
ENTRYPOINT ["/app/bridge"]
EOF

# Build and push
docker build -t bonsai:android-bridge:latest .
docker push docker.io/bonsai/android-bridge:latest
```

### Step 3: Database Setup

```bash
# Create capability registry database
psql postgresql://db:5432/bonsai <<'EOF'
CREATE TABLE capabilities (
    id UUID PRIMARY KEY,
    device_id VARCHAR NOT NULL,
    subject VARCHAR NOT NULL,
    capability VARCHAR NOT NULL,
    issued_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    revoked BOOLEAN DEFAULT FALSE,
    signature BYTEA NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_device_id ON capabilities(device_id);
CREATE INDEX idx_subject ON capabilities(subject);
CREATE INDEX idx_expires_at ON capabilities(expires_at);

-- Cleanup old tokens weekly
CREATE OR REPLACE FUNCTION cleanup_expired_capabilities()
RETURNS void AS $$
BEGIN
    DELETE FROM capabilities WHERE expires_at < NOW() AND revoked = TRUE;
END;
$$ LANGUAGE plpgsql;

-- Schedule cleanup (requires pg_cron extension)
SELECT cron.schedule('cleanup-capabilities', '0 0 * * 0', 'SELECT cleanup_expired_capabilities()');
EOF
```

### Step 4: Deploy to Kubernetes

```bash
# Create namespace and secrets
kubectl create namespace bonsai-android
kubectl create secret generic db-credentials \
  --from-literal=connection-string="postgresql://user:pass@postgres:5432/bonsai" \
  -n bonsai-android

# Apply manifests
kubectl apply -f kubernetes/configmap.yaml
kubectl apply -f kubernetes/deployment.yaml
kubectl apply -f kubernetes/service.yaml
kubectl apply -f kubernetes/hpa.yaml

# Verify deployment
kubectl get pods -n bonsai-android
kubectl logs -f deployment/android-bridge -n bonsai-android
```

## Device Management

### Register Devices

**Via MCP:**
```python
client.tools('android_register_device', {
    'device_id': 'pixel6_001',
    'name': 'Pixel 6 #1',
    'model': 'Pixel 6',
    'api_level': 31,
    'ip': '192.168.1.100',
    'port': 5037,
    'public_key': '<device-ed25519-pk>'
})
```

**Via REST API:**
```bash
curl -X POST http://localhost:8080/api/devices \
  -H "Content-Type: application/json" \
  -d '{
    "device_id": "pixel6_001",
    "name": "Pixel 6 #1",
    "model": "Pixel 6",
    "api_level": 31,
    "ip": "192.168.1.100",
    "port": 5037,
    "public_key": "..."
  }'
```

**Via Config File:**
```yaml
# config/devices.yaml
devices:
  - device_id: pixel6_001
    name: "Pixel 6 #1"
    model: "Pixel 6"
    api_level: 31
    ip: "192.168.1.100"
    port: 5037
    public_key: "ed25519_pk_hex"
    
  - device_id: galaxy_s21_001
    name: "Galaxy S21 #1"
    model: "Galaxy S21"
    api_level: 32
    ip: "192.168.1.101"
    port: 5037
    public_key: "ed25519_pk_hex"
```

### Monitor Device Health

```bash
# Get device list and status
curl http://localhost:8080/api/devices | jq

# Get specific device metrics
curl http://localhost:8080/api/devices/pixel6_001/metrics | jq '{
  screen_frames_sent,
  input_events_processed,
  avg_screen_latency,
  battery_level
}'

# Get connection stats
curl http://localhost:8080/api/health | jq
```

## Monitoring & Observability

### Metrics Collection

**Prometheus Integration:**

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'android-bridge'
    static_configs:
      - targets: ['localhost:9090']
```

**Key Metrics:**

| Metric | Type | Purpose |
|--------|------|---------|
| `bridge_devices_connected` | Gauge | Active connections |
| `bridge_screen_frames_total` | Counter | Total frames sent |
| `bridge_screen_latency_ms` | Histogram | End-to-end latency |
| `bridge_input_events_total` | Counter | Input events |
| `bridge_file_bytes_synced` | Counter | Data transferred |
| `bridge_capability_checks_total` | Counter | Auth checks |
| `bridge_capability_denials_total` | Counter | Failed checks |
| `bridge_errors_total` | Counter | Errors by type |

**W&B Dashboard:**

```python
import wandb

wandb.init(project="bonsai-android-bridge")

metrics = {
    'devices_connected': 42,
    'avg_screen_latency': 45.2,
    'frame_rate': 60,
    'file_sync_rate': 12.5,  # MB/s
    'error_rate': 0.02,  # 2%
    'battery_avg': 75,
}

wandb.log(metrics)
```

### Logging

**Structured Logs:**

```rust
// Enable structured logging
RUST_LOG=bonsai_android_bridge=debug cargo run --release

// Typical log output
[2024-05-31T10:45:23.123Z INFO  bonsai_android_bridge::connection] Device pixel6_001 connected
[2024-05-31T10:45:24.456Z DEBUG bonsai_android_bridge::streaming] Frame #1234: 1080x2400@60fps, latency=42ms
[2024-05-31T10:45:25.789Z WARN  bonsai_android_bridge::security] Capability check failed: device=galaxy_s21 cap=InputInjection
```

**Log Aggregation (ELK Stack):**

```yaml
filebeat.yaml
filebeat.inputs:
- type: log
  paths:
    - /var/log/bonsai-bridge/*.log
  json.message_key: message
  json.keys_under_root: true

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "bonsai-android-bridge"
```

### Alerting

**Alert Rules (Prometheus):**

```yaml
groups:
  - name: android_bridge
    rules:
    - alert: BridgeDown
      expr: up{job="android-bridge"} == 0
      for: 2m
      annotations:
        summary: "Android Bridge is down"

    - alert: HighErrorRate
      expr: rate(bridge_errors_total[5m]) > 0.05
      annotations:
        summary: "Error rate > 5%"

    - alert: DeviceDisconnectRate
      expr: increase(bridge_device_disconnects_total[1h]) > 10
      annotations:
        summary: "More than 10 devices disconnected in 1h"

    - alert: HighScreenLatency
      expr: bridge_screen_latency_ms > 100
      annotations:
        summary: "Screen latency > 100ms"
```

## Operational Procedures

### Emergency Restart

```bash
# Graceful shutdown (5-minute drain)
kubectl patch deployment android-bridge \
  --type='json' -p='[{"op": "replace", "path": "/spec/template/spec/terminationGracePeriodSeconds", "value":300}]'

# Restart
kubectl rollout restart deployment/android-bridge -n bonsai-android

# Monitor rollout
kubectl rollout status deployment/android-bridge -n bonsai-android
```

### Device Reconnection Procedure

```bash
# 1. Stop agent on device
adb shell am stopservice com.bonsai.agent/.BonsaiService

# 2. Clear app cache
adb shell pm clear com.bonsai.agent

# 3. Reinstall agent
adb install -r android-agent/app/build/outputs/apk/release/app-release.apk

# 4. Grant permissions
adb shell pm grant com.bonsai.agent android.permission.BIND_ACCESSIBILITY_SERVICE

# 5. Start service
adb shell am startservice com.bonsai.agent/.BonsaiService

# 6. Reconnect from bridge
curl -X POST http://localhost:8080/api/devices/<device_id>/connect
```

### Capacity Planning

**Per-Device Resource Requirements:**

| Resource | Per Device | For 100 Devices | For 1000 Devices |
|----------|------------|-----------------|------------------|
| Memory | 5 MB | 500 MB | 5 GB |
| CPU (baseline) | ~0.5% | 50% | 500% (5 cores) |
| Network | 30 Mbps @ 60fps | 3 Gbps | 30 Gbps |
| Storage (logs) | ~100 MB/day | 10 GB/day | 100 GB/day |

**Scaling Recommendations:**

```
1-10 devices:
  └─ Single machine (laptop/desktop)
  └─ 4+ GB RAM, 2+ cores
  └─ LAN connectivity

10-100 devices:
  └─ Dedicated server
  └─ 8+ GB RAM, 4+ cores
  └─ Fast network (100 Mbps+)

100-1000 devices:
  └─ Kubernetes cluster
  └─ 10+ nodes, 4+ GB RAM per node
  └─ Distributed network (1+ Gbps)
  └─ PostgreSQL for capability registry
  └─ Load balancer (nginx/HAProxy)
  └─ Distributed logging (ELK/Loki)
  └─ Metrics aggregation (Prometheus/Grafana)

1000+ devices:
  └─ Multi-region deployment
  └─ Global load balancing (GeoDNS)
  └─ Database replication
  └─ Edge caching
```

## Security Operations

### Capability Token Audit

```bash
# List all active capabilities for a device
curl http://localhost:8080/api/devices/pixel6_001/capabilities | jq

# Revoke all capabilities for a device
curl -X POST http://localhost:8080/api/devices/pixel6_001/revoke_all

# Export audit log
curl http://localhost:8080/api/audit-log?device_id=pixel6_001 > audit.json
```

### Incident Response

**Data Breach:**
1. Revoke all capability tokens
2. Disconnect all devices
3. Review access logs in W&B
4. Restart bridge instances with new identity
5. Re-register trusted devices

**Unauthorized Access Detected:**
1. Isolate device (disconnect network)
2. Review telemetry events
3. Check for suspicious input/file operations
4. Revoke compromised agent tokens
5. Investigate source of capability tokens

## Troubleshooting

### Device Won't Connect

```bash
# 1. Check device is on network
ping 192.168.1.100

# 2. Check bridge service is running
ps aux | grep bonsai-android-bridge

# 3. Check port is listening
netstat -tlnp | grep 5037

# 4. Check device agent is running
adb logcat | grep BonsaiAgent

# 5. Check firewall rules
sudo iptables -L | grep 5037

# 6. Review bridge logs
kubectl logs -f deployment/android-bridge -n bonsai-android
```

### High Screen Latency

```bash
# 1. Check network bandwidth
iperf3 -c device_ip -t 10

# 2. Monitor bitrate adaptation
curl http://localhost:8080/api/devices/<id>/streaming-stats | jq .bitrate

# 3. Check device CPU
adb shell top -n 1 | head -20

# 4. Reduce stream resolution
curl -X POST http://localhost:8080/api/devices/<id>/config \
  -d '{"screen_width": 720, "screen_height": 1280}'
```

### Capability Check Failures

```bash
# List tokens for a subject
curl http://localhost:8080/api/capabilities?subject=agent1 | jq

# Check token validity
curl http://localhost:8080/api/capabilities/<token_id>/validate | jq

# Re-issue capability
curl -X POST http://localhost:8080/api/capabilities \
  -d '{
    "device_id": "pixel6_001",
    "subject": "agent1",
    "capability": "ScreenStream",
    "duration_hours": 24
  }'
```

## Maintenance

### Regular Tasks

**Daily:**
- Monitor error rates and alerts
- Check device connection stability
- Review security logs

**Weekly:**
- Run database maintenance
- Analyze performance trends
- Update agent APK if needed

**Monthly:**
- Security audit (capability tokens)
- Capacity planning review
- Dependency updates

**Quarterly:**
- Major version upgrades
- Security assessment
- Disaster recovery drill

## Disaster Recovery

### Backup Strategy

```bash
# Backup capability registry (daily)
pg_dump postgresql://db:5432/bonsai | gzip > backups/capabilities-$(date +%Y%m%d).sql.gz

# Backup device configuration (daily)
kubectl get configmap bridge-config -o yaml > backups/config-$(date +%Y%m%d).yaml

# Backup telemetry (weekly)
# W&B provides automatic backup
```

### Recovery Procedures

**Lost Capability Registry:**
1. Restore from backup: `gunzip < backups/capabilities-YYYYMMDD.sql.gz | psql`
2. Reissue capabilities to known subjects
3. Notify users of affected access

**Bridge Instance Failure:**
1. Kubernetes automatically restarts pod
2. Devices reconnect to load balancer
3. No data loss (stateless design)
4. Service unavailability: ~30 seconds

**Cascading Device Failures:**
1. Check network connectivity
2. Review recent config changes
3. Check capability token expiration
4. Redeploy agent APK if needed
5. Restart affected devices one-by-one

## Conclusion

The Android Bridge is designed for production operation at scale. Follow these procedures for reliable, secure device management at 1-1000+ device scale.
