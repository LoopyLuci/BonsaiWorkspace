# Deployment Guide - Production Setup

**Deploy Omnisystem applications to production**

---

## Pre-Deployment Checklist

- [ ] Code reviewed and tested
- [ ] Security scan passed
- [ ] Performance benchmarked
- [ ] Documentation updated
- [ ] Backup strategy in place
- [ ] Monitoring configured
- [ ] Runbooks created
- [ ] Team trained

---

## Build for Production

### Compile with Optimizations
```bash
omnisystem build --release --optimize 3
# Creates: target/release/omnisystem-app
```

### Generate Build Artifacts
```bash
omnisystem build --release --output ./dist
ls dist/
# omnisystem-app (executable)
# omnisystem-app.bin (bytecode)
# omnisystem-app.onnx (model export, if SYLVA)
```

---

## Deployment Methods

### Docker Deployment
```dockerfile
FROM omnisystem:latest

COPY dist/ /app/
WORKDIR /app

RUN omnisystem module load base-modules/MODULE_MANIFEST.omni

EXPOSE 8080
CMD ["omnisystem", "run", "app.ti"]
```

```bash
docker build -t my-app:1.0 .
docker push my-app:1.0
docker run -p 8080:8080 my-app:1.0
```

### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: omnisystem-app
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: app
        image: my-app:1.0
        ports:
        - containerPort: 8080
        resources:
          requests:
            memory: "256Mi"
            cpu: "500m"
          limits:
            memory: "512Mi"
            cpu: "1000m"
```

### Bare Metal Deployment
```bash
# Copy binary to production server
scp dist/omnisystem-app user@prod-server:/opt/app/

# Set up service
sudo cp omnisystem.service /etc/systemd/system/
sudo systemctl enable omnisystem
sudo systemctl start omnisystem
```

---

## Configuration

### Environment Variables
```bash
# Database
export OMNISYSTEM_DB=postgresql://localhost/mydb

# Cluster
export OMNISYSTEM_NODES=node1,node2,node3

# Security
export OMNISYSTEM_KEY_PATH=/secure/keys/
export OMNISYSTEM_TLS_CERT=/secure/certs/

# Monitoring
export OMNISYSTEM_METRICS_ENDPOINT=localhost:9090
```

### Configuration File
```toml
[server]
port = 8080
workers = 4

[cluster]
nodes = ["127.0.0.1:5001", "127.0.0.1:5002"]
consensus = "raft"
replicas = 3

[security]
tls = true
cert = "/secure/cert.pem"
key = "/secure/key.pem"

[monitoring]
enable = true
endpoint = "localhost:9090"
```

---

## Migration Strategy

### Blue-Green Deployment
```
Production (Blue)  →  New Version (Green)
     ↓
Test Green
     ↓
Switch traffic to Green
     ↓
Blue becomes backup
```

### Canary Deployment
```
10% traffic to v2
Monitor metrics
↓
50% traffic to v2
↓
100% traffic to v2
```

---

## Health Checks

### Liveness Probe
```
GET /health/live
Response: 200 OK → Alive
Response: 503 → Dead, restart
```

### Readiness Probe
```
GET /health/ready
Response: 200 OK → Ready for traffic
Response: 503 → Warming up, wait
```

### Configuration
```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

---

## Database Migrations

### Schema Migrations
```bash
omnisystem migrate --version latest
omnisystem migrate --check  # Verify before applying
```

### Data Migrations
```bash
omnisystem migrate data --script migrations/001_add_column.sql
```

---

## Backup & Recovery

### Backup Strategy
```bash
# Daily backups
0 2 * * * omnisystem backup --full /backups/full

# Hourly incremental
0 * * * * omnisystem backup --incremental /backups/incremental
```

### Recovery
```bash
# List backups
omnisystem backup list

# Restore from backup
omnisystem restore --from /backups/full/2026-06-15.backup

# Verify recovery
omnisystem verify --backup-integrity
```

---

## Monitoring Setup

### Install Prometheus
```bash
docker run -d -p 9090:9090 prom/prometheus
```

### Scrape Configuration
```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'omnisystem'
    static_configs:
      - targets: ['localhost:8080']
```

### Key Metrics
```
omnisystem_requests_total
omnisystem_request_duration_seconds
omnisystem_error_total
omnisystem_memory_usage_bytes
omnisystem_cluster_nodes_healthy
```

---

## Alerting

### Alert Rules
```yaml
groups:
  - name: omnisystem
    rules:
      - alert: HighErrorRate
        expr: rate(omnisystem_error_total[5m]) > 0.1
        for: 5m

      - alert: ClusterUnhealthy
        expr: omnisystem_cluster_nodes_healthy < 2
        for: 1m
```

---

## Rollback Plan

### Quick Rollback
```bash
# If deployment fails
kubectl rollout undo deployment/omnisystem-app

# If version is bad
docker pull my-app:previous
docker run -p 8080:8080 my-app:previous
```

### Data Rollback
```bash
omnisystem restore --from /backups/pre-deploy.backup
```

---

## Post-Deployment

### Smoke Tests
```bash
omnisystem test --integration
curl http://localhost:8080/health
```

### Performance Validation
```bash
omnisystem bench --against baseline.txt
```

### Security Scan
```bash
omnisystem security scan --post-deploy
```

---

## Scaling

### Vertical Scaling
```bash
# Increase CPU/Memory
docker update --cpus 2 --memory 1g <container>
```

### Horizontal Scaling
```bash
# Add more replicas
kubectl scale deployment omnisystem-app --replicas=5
```

---

## Troubleshooting Deployment

| Issue | Solution |
|-------|----------|
| App won't start | Check logs, verify config |
| High memory | Profile, reduce cache size |
| Slow startup | Warm up caches, async init |
| Connection failures | Check network, firewall |

---

## Next Steps

- Monitoring: [OPERATIONS.md](OPERATIONS.md)
- Troubleshooting: [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- Performance: [PERFORMANCE.md](PERFORMANCE.md)

---

**Deployment** - Get Omnisystem to production safely!
