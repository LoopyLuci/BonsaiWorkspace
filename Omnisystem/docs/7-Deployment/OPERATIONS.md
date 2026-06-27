# Operations & Maintenance Guide

**Monitor and maintain Omnisystem applications in production**

---

## Monitoring

### Key Metrics

**System Metrics:**
- CPU usage (target <70%)
- Memory usage (target <80%)
- Disk usage (target <85%)
- Network latency (target <100ms)

**Application Metrics:**
- Request latency (p50, p95, p99)
- Error rate (target <0.1%)
- Throughput (requests/sec)
- Queue depth

**Cluster Metrics (AETHER):**
- Cluster health (all nodes up)
- Replication lag (target <100ms)
- Consensus latency (target <50ms)
- Message count (track trends)

### Setup Monitoring

```bash
# Start monitoring agent
omnisystem monitor --export prometheus:9090

# Query metrics
curl http://localhost:9090/metrics
```

---

## Logging

### Log Levels
```bash
# Set global log level
export OMNISYSTEM_LOG_LEVEL=info

# Levels: debug, info, warn, error, critical
omnisystem run --log-level debug
```

### Log Output
```bash
# Send to file
omnisystem run --log-file app.log

# Send to syslog
omnisystem run --log-syslog

# JSON format for parsing
omnisystem run --log-format json
```

---

## Health Checks

### Manual Health Check
```bash
curl http://localhost:8080/health
# Response: {"status": "healthy"}

curl http://localhost:8080/health/deep
# Response: detailed health info
```

### Automated Health Checks
```bash
omnisystem health-check --interval 30s
```

---

## Maintenance Tasks

### Daily Tasks
- [ ] Monitor error rates
- [ ] Check disk space
- [ ] Review logs for warnings
- [ ] Verify backups completed

### Weekly Tasks
- [ ] Review metrics trends
- [ ] Update dependencies
- [ ] Run security scan
- [ ] Test failover procedures

### Monthly Tasks
- [ ] Full system audit
- [ ] Capacity planning
- [ ] Performance optimization
- [ ] Documentation update

---

## Cluster Management (AETHER)

### Add Node
```bash
omnisystem cluster add-node --id node4 --address 127.0.0.1:5004
```

### Remove Node
```bash
omnisystem cluster remove-node --id node4
```

### Check Status
```bash
omnisystem cluster status
# Shows: Leader, replicas, replication lag, consensus status
```

### Rebalance
```bash
omnisystem cluster rebalance --target-shards 16
```

---

## Backup & Recovery

### Create Backup
```bash
# Full backup
omnisystem backup create --type full --output backup.tar.gz

# Incremental backup
omnisystem backup create --type incremental --since last --output backup-inc.tar.gz
```

### List Backups
```bash
omnisystem backup list
# Shows backup date, size, type
```

### Restore from Backup
```bash
omnisystem backup restore --from backup.tar.gz --verify
```

---

## Performance Monitoring

### Real-Time Metrics
```bash
omnisystem metrics --real-time
# Shows CPU, memory, requests/sec, latency
```

### Historical Trends
```bash
omnisystem metrics --export --start 2026-06-01 --end 2026-06-15
```

### Performance Bottleneck Detection
```bash
omnisystem profile --detect-bottlenecks
# Automatically identifies slow operations
```

---

## Alerting

### Alert Rules
```yaml
alerts:
  - name: HighErrorRate
    threshold: error_rate > 0.01
    action: page

  - name: ClusterUnhealthy
    threshold: healthy_nodes < 2
    action: page

  - name: HighLatency
    threshold: p99_latency > 1000ms
    action: notify
```

### Send Alert
```bash
omnisystem alert send --level critical --message "System down"
```

---

## Configuration Management

### Apply Configuration
```bash
omnisystem config apply --file config.toml
```

### Hot Reload
```bash
omnisystem config reload --hot
# Does not restart service
```

### Verify Configuration
```bash
omnisystem config validate --file config.toml
```

---

## Log Rotation

### Configure Rotation
```toml
[logging]
rotation = "daily"
max-files = 30
max-size-mb = 100
compress = true
```

### Manual Rotation
```bash
omnisystem logs rotate
```

---

## Database Maintenance

### Vacuum Database
```bash
omnisystem db vacuum
```

### Analyze Statistics
```bash
omnisystem db analyze
```

### Repair Database
```bash
omnisystem db repair --check-integrity
```

---

## Upgrade Path

### Check for Updates
```bash
omnisystem update check
# Shows available versions
```

### Staged Upgrade
```bash
# 1. Download new version
omnisystem update download --version 2.1.0

# 2. Test in staging
omnisystem update test --version 2.1.0

# 3. Deploy to production
omnisystem update deploy --version 2.1.0 --blue-green
```

---

## Incident Response

### Declare Incident
```bash
omnisystem incident declare --severity critical --title "Database down"
```

### Get System State
```bash
omnisystem incident diagnostics > incident.log
# Captures: logs, metrics, config, system state
```

### Remediate
```bash
omnisystem incident remediate --action restart-service
# Automatic remediation
```

---

## Runbooks

### Common Runbook: Service Down
1. Check logs: `omnisystem logs tail --lines 100`
2. Check health: `curl /health`
3. Restart: `omnisystem restart`
4. Verify: `curl /health`

### Common Runbook: High Memory
1. Check process: `omnisystem metrics | grep memory`
2. Check leaks: `omnisystem profile --memory`
3. Clear cache: `omnisystem cache clear`
4. Restart if needed: `omnisystem restart`

---

## Documentation

### Runbook Checklist
- [ ] Title and impact
- [ ] Prerequisites
- [ ] Step-by-step instructions
- [ ] Verification steps
- [ ] Rollback procedure
- [ ] Escalation path

---

## Next Steps

- Troubleshooting: [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- Tuning: [TUNING.md](TUNING.md)
- Deployment: [DEPLOYMENT.md](DEPLOYMENT.md)

---

**Operations** - Keep Omnisystem running smoothly!
