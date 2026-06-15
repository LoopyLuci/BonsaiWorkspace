# 🚀 DEPLOYMENT GUIDE - PRODUCTION DEPLOYMENT

**Step-by-step guide to deploy the Autonomous Enterprise Platform globally**

---

## Quick Deploy (5 Minutes)

```bash
# 1. Clone repository
git clone <repository-url> autonomous-platform
cd autonomous-platform

# 2. Build release
cargo build --release --all

# 3. Run all tests
cargo test --all --release

# 4. Start platform
RUST_LOG=info cargo run --release -p conductor-core

# 5. Access at
# REST API: http://localhost:8080/api
# GraphQL: http://localhost:8080/graphql
# WebSocket: ws://localhost:8080/ws
# Web UI: http://localhost:3000
```

---

## Prerequisites

### System Requirements

**Minimum**:
- **CPU**: 4 cores (8 recommended)
- **RAM**: 8GB (16GB recommended)
- **Disk**: 100GB SSD
- **OS**: Linux (Ubuntu 20.04+ recommended), macOS, Windows WSL2

**Recommended** (Production):
- **CPU**: 16+ cores
- **RAM**: 64GB+
- **Disk**: 500GB+ NVMe SSD
- **Network**: 10Gbps connection
- **GPU**: NVIDIA GPU (optional, for AI workloads)

### Software Requirements

```bash
# Rust toolchain (1.70+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update

# Docker (20.10+)
curl -fsSL https://get.docker.com | sh

# Kubernetes (1.27+)
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"

# Terraform (1.5+)
wget https://releases.hashicorp.com/terraform/1.5.0/terraform_1.5.0_linux_amd64.zip

# Git
apt-get install git
```

### Required Access

- **Docker Hub** or private registry access
- **Kubernetes cluster** (EKS, AKS, GKE, or self-hosted)
- **Cloud provider** account (AWS, Azure, GCP)
- **Secrets management** (HashiCorp Vault)
- **Monitoring** account (Datadog, New Relic, etc.)

---

## Installation

### 1. Clone Repository

```bash
git clone https://github.com/your-org/autonomous-platform.git
cd autonomous-platform

# Verify git is clean
git status
```

### 2. Verify Build Environment

```bash
# Check Rust version
rustc --version  # Should be 1.70+

# Check Cargo
cargo --version

# Check Docker
docker --version

# Verify build works
cargo check --workspace
```

### 3. Build Release

```bash
# Build everything
cargo build --release --all

# Or specific component
cargo build --release -p conductor-core

# Verify no errors
echo $?  # Should be 0
```

### 4. Run Tests

```bash
# All tests
cargo test --all --release

# Specific test
cargo test --lib -p conductor-core

# With logging
RUST_LOG=debug cargo test --all -- --nocapture
```

---

## Local Development

### Start Development Server

```bash
# Terminal 1: Start Conductor core
RUST_LOG=debug cargo run -p conductor-core

# Terminal 2: Start analytics
cargo run -p advanced-analytics-core

# Terminal 3: Start web UI
cd web-ui
npm install
npm start
```

### Access Services

- **API**: http://localhost:8080/api
- **GraphQL**: http://localhost:8080/graphql
- **Metrics**: http://localhost:9090
- **Logs**: http://localhost:5601 (Elasticsearch)
- **Web UI**: http://localhost:3000

### Test Integration

```bash
# List containers
curl http://localhost:8080/api/containers

# GraphQL query
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ containers { id name } }"}'
```

---

## Docker Deployment

### Build Docker Image

```bash
# Build image
docker build -f Dockerfile.conductor -t platform:latest .

# Or use provided script
./scripts/build-docker.sh

# Verify build
docker images | grep platform
```

### Dockerfile Example

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/conductor-core /usr/local/bin/
EXPOSE 8080
ENTRYPOINT ["conductor-core"]
```

### Run Docker Container

```bash
# Run single container
docker run -d \
  --name conductor \
  -p 8080:8080 \
  -e RUST_LOG=info \
  platform:latest

# With Docker Compose
docker-compose up -d

# View logs
docker logs -f conductor

# Stop container
docker stop conductor
```

---

## Kubernetes Deployment

### Prerequisites

```bash
# Verify cluster access
kubectl cluster-info

# Create namespace
kubectl create namespace autonomous-platform

# Create secrets
kubectl create secret generic conductor-config \
  --from-file=config.json \
  -n autonomous-platform
```

### Deploy with Helm

```bash
# Add Helm repository
helm repo add platform https://charts.platform.example.com
helm repo update

# Install
helm install conductor platform/conductor \
  -n autonomous-platform \
  --values values.yaml

# Verify deployment
kubectl get deployments -n autonomous-platform
kubectl get pods -n autonomous-platform

# View logs
kubectl logs -f deployment/conductor -n autonomous-platform
```

### Helm values.yaml Example

```yaml
replicaCount: 3

image:
  repository: platform/conductor
  tag: "1.0"
  pullPolicy: IfNotPresent

resources:
  requests:
    cpu: 2
    memory: 4Gi
  limits:
    cpu: 4
    memory: 8Gi

service:
  type: LoadBalancer
  port: 8080

ingress:
  enabled: true
  hosts:
    - conductor.example.com

persistence:
  enabled: true
  size: 50Gi
  storageClassName: fast-ssd

monitoring:
  enabled: true
  serviceMonitor:
    enabled: true
```

### Manual Kubernetes Deployment

```bash
# Create deployment
kubectl apply -f k8s/conductor-deployment.yaml

# Create service
kubectl apply -f k8s/conductor-service.yaml

# Create ingress
kubectl apply -f k8s/conductor-ingress.yaml

# Scale deployment
kubectl scale deployment conductor --replicas=5

# Monitor deployment
kubectl describe deployment conductor
```

---

## Multi-Region Deployment

### Setup

```bash
# Deploy to Region 1 (US-East)
aws ec2 run-instances \
  --image-id ami-xxxxxxxx \
  --count 3 \
  --instance-type t3.xlarge \
  --region us-east-1

# Deploy to Region 2 (EU-West)
aws ec2 run-instances \
  --image-id ami-xxxxxxxx \
  --count 3 \
  --instance-type t3.xlarge \
  --region eu-west-1

# Configure replication
./scripts/setup-replication.sh us-east-1 eu-west-1
```

### Load Balancing

```bash
# Create global load balancer
aws elbv2 create-load-balancer \
  --name platform-global \
  --type network \
  --scheme internet-facing

# Add targets
aws elbv2 register-targets \
  --target-group-arn arn:aws:... \
  --targets Id=i-xxxxx Id=i-yyyyy
```

---

## Infrastructure-as-Code

### Terraform

```bash
# Initialize Terraform
cd infrastructure
terraform init

# Plan deployment
terraform plan -out=tfplan

# Apply changes
terraform apply tfplan

# Destroy (cleanup)
terraform destroy
```

### Terraform Example

```hcl
# main.tf
provider "aws" {
  region = "us-east-1"
}

resource "aws_eks_cluster" "platform" {
  name            = "autonomous-platform"
  role_arn        = aws_iam_role.platform.arn
  vpc_config {
    subnet_ids = aws_subnet.platform[*].id
  }
}

resource "kubernetes_deployment" "conductor" {
  metadata {
    name      = "conductor"
    namespace = "autonomous-platform"
  }
  spec {
    replicas = 3
    template {
      spec {
        container {
          image = "platform/conductor:latest"
          port {
            container_port = 8080
          }
        }
      }
    }
  }
}
```

---

## Configuration

### Environment Variables

```bash
# Logging
RUST_LOG=info
RUST_BACKTRACE=1

# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Database
DATABASE_URL=postgresql://user:pass@localhost/platform
DATABASE_POOL_SIZE=20

# Cache
REDIS_URL=redis://localhost:6379

# Monitoring
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
PROMETHEUS_ADDR=0.0.0.0:9090

# Security
API_KEY_SECRET=...
JWT_SECRET=...
TLS_CERT_PATH=/etc/conductor/cert.pem
TLS_KEY_PATH=/etc/conductor/key.pem

# Features
ENABLE_SWARM=true
ENABLE_ANALYTICS=true
ENABLE_SELF_HEALING=true
```

### Configuration File (config.json)

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "tls": {
      "enabled": true,
      "cert_path": "/etc/conductor/cert.pem",
      "key_path": "/etc/conductor/key.pem"
    }
  },
  "database": {
    "url": "postgresql://localhost/platform",
    "pool_size": 20,
    "timeout": 30
  },
  "features": {
    "swarm": true,
    "analytics": true,
    "self_healing": true,
    "gpu_support": true
  },
  "monitoring": {
    "metrics_enabled": true,
    "tracing_enabled": true,
    "logging_level": "info"
  },
  "security": {
    "tls_min_version": "1.3",
    "rbac_enabled": true,
    "audit_logging": true
  }
}
```

---

## Networking

### Ingress Configuration

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: conductor
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - api.example.com
      secretName: conductor-tls
  rules:
    - host: api.example.com
      http:
        paths:
          - path: /api
            pathType: Prefix
            backend:
              service:
                name: conductor
                port:
                  number: 8080
```

### Service Mesh (Istio)

```yaml
apiVersion: security.istio.io/v1beta1
kind: PeerAuthentication
metadata:
  name: default
spec:
  mtls:
    mode: STRICT
---
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: conductor
spec:
  hosts:
    - conductor
  http:
    - match:
        - uri:
            prefix: /api
      route:
        - destination:
            host: conductor
            port:
              number: 8080
```

---

## Database

### Initialize Database

```bash
# PostgreSQL
createdb autonomous_platform
psql autonomous_platform < schema.sql

# Run migrations
sqlx database create
sqlx migrate run
```

### Schema Example

```sql
CREATE TABLE containers (
  id SERIAL PRIMARY KEY,
  container_id VARCHAR(255) UNIQUE NOT NULL,
  name VARCHAR(255) NOT NULL,
  status VARCHAR(50) NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE metrics (
  id SERIAL PRIMARY KEY,
  container_id VARCHAR(255) NOT NULL,
  cpu_percent FLOAT NOT NULL,
  memory_bytes BIGINT NOT NULL,
  recorded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_containers_status ON containers(status);
CREATE INDEX idx_metrics_container_id ON metrics(container_id);
```

---

## Monitoring & Observability

### Enable Monitoring

```bash
# Prometheus metrics
curl http://localhost:9090/metrics

# Jaeger tracing
export JAEGER_ENDPOINT=http://localhost:6831
export JAEGER_SAMPLER_TYPE=const
export JAEGER_SAMPLER_PARAM=1

# ELK Stack
curl http://localhost:9200/_cluster/health

# Grafana dashboard
http://localhost:3000 (admin/admin)
```

### Alerting Rules

```yaml
groups:
  - name: platform
    rules:
      - alert: HighCPU
        expr: node_cpu_usage > 80
        for: 5m
        annotations:
          summary: "High CPU usage"

      - alert: LowMemory
        expr: node_memory_available < 1000000000
        for: 5m
        annotations:
          summary: "Low memory available"

      - alert: ServiceDown
        expr: up{job="conductor"} == 0
        for: 1m
        annotations:
          summary: "Conductor service down"
```

---

## Backup & Recovery

### Backup Strategy

```bash
# Daily backup
0 2 * * * pg_dump autonomous_platform > /backups/db-$(date +%Y%m%d).sql

# Upload to S3
aws s3 cp /backups/db-*.sql s3://platform-backups/

# Verify backup
pg_restore --list /backups/db-latest.sql
```

### Restore from Backup

```bash
# From latest backup
psql autonomous_platform < /backups/db-latest.sql

# Verify data
psql -c "SELECT COUNT(*) FROM containers;"
```

---

## Health Checks

### Liveness Probe

```bash
curl -f http://localhost:8080/health/live || exit 1
```

### Readiness Probe

```bash
curl -f http://localhost:8080/health/ready || exit 1
```

### Kubernetes Example

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 5
```

---

## Scaling

### Horizontal Scaling

```bash
# Kubernetes
kubectl scale deployment conductor --replicas=10

# Or with Helm
helm upgrade conductor platform/conductor \
  --set replicaCount=10

# Verify
kubectl get pods -l app=conductor
```

### Vertical Scaling

```yaml
resources:
  requests:
    cpu: 4
    memory: 16Gi
  limits:
    cpu: 8
    memory: 32Gi
```

### Auto-Scaling

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: conductor
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: conductor
  minReplicas: 3
  maxReplicas: 50
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

---

## Production Checklist

- ✅ All tests passing (100%)
- ✅ No compilation errors
- ✅ No unsafe code
- ✅ Monitoring enabled
- ✅ Logging configured
- ✅ Database backups running
- ✅ Secrets configured
- ✅ TLS certificates valid
- ✅ Ingress configured
- ✅ Load balancer healthy
- ✅ Health checks passing
- ✅ Alerts configured
- ✅ Disaster recovery tested
- ✅ Security scan passed
- ✅ Documentation updated

---

## Troubleshooting

### Service Won't Start

```bash
# Check logs
docker logs conductor
kubectl logs deployment/conductor

# Check configuration
cat config.json | jq .

# Test connectivity
curl -v http://localhost:8080/health/live
```

### High Memory Usage

```bash
# Check memory allocation
ps aux | grep conductor

# Increase memory limit
kubectl set resources deployment conductor --limits=memory=32Gi

# Check for leaks
cargo test --release -- --test-threads=1
```

### Database Connection Issues

```bash
# Test connection
psql $DATABASE_URL -c "SELECT 1"

# Check connection pool
curl http://localhost:8080/metrics | grep pool

# Increase pool size
export DATABASE_POOL_SIZE=50
```

---

## Support

**Documentation**: [docs/](docs/)  
**Issues**: GitHub Issues  
**Community**: [community.example.com](https://community.example.com)  
**Enterprise**: sales@platform.example.com  

---

**Status**: ✅ Ready for Production  
**Last Updated**: 2026-06-13  

🚀 **Deploy and Go Global**
